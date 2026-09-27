//! External-browser login flow (S-05) on top of the PKCE primitives.
//!
//! One attempt:
//! 1. bind a loopback listener on a random port (127.0.0.1 only),
//! 2. hand the caller an authorize URL carrying the S256 challenge and state,
//! 3. wait for exactly one callback, validate it (endpoint, state, error, code),
//! 4. exchange the code with the verifier at the token endpoint.
//!
//! The attempt ends on the first callback, on `cancel`, or when `timeout`
//! passes; the listener is closed in every case, so a late or replayed callback
//! finds nothing to talk to. The verifier stays in this process and is sent
//! only to the token endpoint.
//!
//! Opening the system browser and registering the loopback redirect with the
//! identity provider are left to the caller; the redirect allowlist entry is an
//! owner decision, so no UI starts this flow yet.
#![allow(dead_code)] // Wired into the login screen once the redirect allowlist is approved.

use std::time::Duration;

use serde::Deserialize;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
};
use url::Url;

use crate::pkce::{self, CallbackError, PkceAttempt};

/// Longest request line we read from the loopback callback.
const MAX_REQUEST_BYTES: usize = 8 * 1024;
const CALLBACK_PATH: &str = "/callback";

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum LoginError {
    /// No callback arrived before the deadline.
    TimedOut,
    /// The user (or the app) stopped the attempt.
    Cancelled,
    /// The callback did not belong to this attempt or was malformed.
    Callback(CallbackError),
    /// The token endpoint refused the code or could not be reached.
    Exchange(String),
    /// The loopback listener could not be opened.
    Listener(String),
}

/// Tokens the provider returned for the exchanged code.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct LoginTokens {
    pub(crate) access_token: String,
    #[serde(default)]
    pub(crate) refresh_token: Option<String>,
    #[serde(default)]
    pub(crate) expires_in: Option<u64>,
}

/// A started attempt: open `authorize_url` in the browser, then `finish`.
pub(crate) struct PendingLogin {
    pub(crate) authorize_url: Url,
    redirect: Url,
    attempt: PkceAttempt,
    listener: TcpListener,
}

/// Stops a pending attempt from another task (e.g. a "cancel" button).
pub(crate) struct CancelHandle(Option<oneshot::Sender<()>>);

impl CancelHandle {
    pub(crate) fn cancel(mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
    }
}

/// Binds the loopback listener and builds the authorize URL.
pub(crate) async fn start(auth_base: &Url, provider: &str) -> Result<PendingLogin, LoginError> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| LoginError::Listener(error.to_string()))?;
    let port = listener
        .local_addr()
        .map_err(|error| LoginError::Listener(error.to_string()))?
        .port();
    let redirect = Url::parse(&format!("http://127.0.0.1:{port}{CALLBACK_PATH}"))
        .map_err(|error| LoginError::Listener(error.to_string()))?;
    let attempt = pkce::new_attempt().map_err(LoginError::Listener)?;
    let authorize_url = pkce::authorize_url(auth_base, provider, &redirect, &attempt)
        .map_err(LoginError::Listener)?;
    Ok(PendingLogin {
        authorize_url,
        redirect,
        attempt,
        listener,
    })
}

impl PendingLogin {
    pub(crate) fn cancel_handle(&mut self) -> (CancelHandle, oneshot::Receiver<()>) {
        let (sender, receiver) = oneshot::channel();
        (CancelHandle(Some(sender)), receiver)
    }

    /// Waits for the callback, validates it and exchanges the code.
    pub(crate) async fn finish(
        self,
        token_endpoint: &Url,
        api_key: &str,
        timeout: Duration,
        cancelled: oneshot::Receiver<()>,
    ) -> Result<LoginTokens, LoginError> {
        let PendingLogin {
            redirect,
            attempt,
            listener,
            ..
        } = self;
        let callback = tokio::select! {
            result = accept_callback(&listener, &redirect) => result?,
            () = tokio::time::sleep(timeout) => return Err(LoginError::TimedOut),
            _ = cancelled => return Err(LoginError::Cancelled),
        };
        // One callback per attempt: the listener closes here.
        drop(listener);
        let code = pkce::validate_callback(&callback, &redirect, &attempt.state)
            .map_err(LoginError::Callback)?;
        exchange(token_endpoint, api_key, &code, &attempt.verifier).await
    }
}

/// Reads one HTTP request line from the loopback socket, answers with a short
/// page, and returns the full callback URL.
async fn accept_callback(listener: &TcpListener, redirect: &Url) -> Result<Url, LoginError> {
    let (mut socket, _) = listener
        .accept()
        .await
        .map_err(|error| LoginError::Listener(error.to_string()))?;
    let mut buffer = vec![0_u8; MAX_REQUEST_BYTES];
    let mut read = 0;
    while read < buffer.len() {
        let count = socket
            .read(&mut buffer[read..])
            .await
            .map_err(|error| LoginError::Listener(error.to_string()))?;
        if count == 0 {
            break;
        }
        read += count;
        if buffer[..read]
            .windows(4)
            .any(|window| window == b"\r\n\r\n")
        {
            break;
        }
    }
    let request = String::from_utf8_lossy(&buffer[..read]);
    let target = request
        .lines()
        .next()
        .and_then(|line| {
            let mut parts = line.split(' ');
            match (parts.next(), parts.next()) {
                (Some("GET"), Some(target)) => Some(target.to_owned()),
                _ => None,
            }
        })
        .ok_or(LoginError::Callback(CallbackError::WrongRedirect))?;
    let page = "<!doctype html><meta charset=utf-8><title>School Collect</title>\
                <p>로그인 결과를 앱으로 보냈습니다. 이 창을 닫아도 됩니다.</p>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\
         Cache-Control: no-store\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
        page.len()
    );
    let _ = socket.write_all(response.as_bytes()).await;
    let _ = socket.shutdown().await;
    redirect
        .join(&target)
        .map_err(|_| LoginError::Callback(CallbackError::WrongRedirect))
}

/// Exchanges the authorization code with the PKCE verifier (Supabase Auth
/// `grant_type=pkce`).
async fn exchange(
    token_endpoint: &Url,
    api_key: &str,
    code: &str,
    verifier: &str,
) -> Result<LoginTokens, LoginError> {
    if token_endpoint.scheme() != "https" && token_endpoint.host_str() != Some("127.0.0.1") {
        return Err(LoginError::Exchange(
            "the token endpoint must use https (http only for a loopback test provider)".into(),
        ));
    }
    let mut url = token_endpoint.clone();
    url.query_pairs_mut().append_pair("grant_type", "pkce");
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|error| LoginError::Exchange(error.to_string()))?;
    let response = client
        .post(url)
        .header("apikey", api_key)
        .json(&serde_json::json!({ "auth_code": code, "code_verifier": verifier }))
        .send()
        .await
        .map_err(|_| LoginError::Exchange("the token endpoint could not be reached".into()))?;
    let status = response.status();
    if !status.is_success() {
        // The body may echo request details, so only the status is reported.
        return Err(LoginError::Exchange(format!(
            "token endpoint returned {}",
            status.as_u16()
        )));
    }
    let tokens: LoginTokens = response
        .json()
        .await
        .map_err(|_| LoginError::Exchange("token response was not understood".into()))?;
    if tokens.access_token.trim().is_empty() {
        return Err(LoginError::Exchange(
            "token response had no access token".into(),
        ));
    }
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        extract::{Query, State},
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
        routing::post,
    };
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    /// What the fake provider saw on its token endpoint.
    #[derive(Clone, Default)]
    struct Seen(Arc<Mutex<Vec<serde_json::Value>>>);

    /// Fake identity provider: `/token?grant_type=pkce` accepts only the code
    /// `good-code` with a verifier whose S256 challenge was registered.
    async fn fake_provider(challenge: Arc<Mutex<String>>) -> (Url, Seen) {
        let seen = Seen::default();
        async fn token(
            State((challenge, seen)): State<(Arc<Mutex<String>>, Seen)>,
            Query(query): Query<HashMap<String, String>>,
            headers: HeaderMap,
            Json(body): Json<serde_json::Value>,
        ) -> impl IntoResponse {
            seen.0.lock().unwrap().push(body.clone());
            let verifier = body["code_verifier"].as_str().unwrap_or_default();
            let ok = query.get("grant_type").map(String::as_str) == Some("pkce")
                && headers.get("apikey").is_some()
                && body["auth_code"] == "good-code"
                && pkce::challenge_for(verifier) == *challenge.lock().unwrap();
            if ok {
                (
                    StatusCode::OK,
                    Json(serde_json::json!({
                        "access_token": "access-from-fake",
                        "refresh_token": "refresh-from-fake",
                        "expires_in": 3600
                    })),
                )
                    .into_response()
            } else {
                (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": "invalid_grant" })),
                )
                    .into_response()
            }
        }
        let app = Router::new()
            .route("/auth/v1/token", post(token))
            .with_state((challenge, seen.clone()));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (
            Url::parse(&format!("http://{address}/auth/v1/token")).unwrap(),
            seen,
        )
    }

    fn auth_base() -> Url {
        Url::parse("https://project.example.test/auth/v1/").unwrap()
    }

    /// The redirect and state the authorize URL asked the provider to use.
    fn redirect_and_state(authorize: &Url) -> (Url, String) {
        let redirect_to = authorize
            .query_pairs()
            .find(|(key, _)| key == "redirect_to")
            .map(|(_, value)| value.into_owned())
            .unwrap();
        let redirect = Url::parse(&redirect_to).unwrap();
        let state = redirect
            .query_pairs()
            .find(|(key, _)| key == "state")
            .map(|(_, value)| value.into_owned())
            .unwrap();
        let mut bare = redirect.clone();
        bare.set_query(None);
        (bare, state)
    }

    fn challenge_of(authorize: &Url) -> String {
        authorize
            .query_pairs()
            .find(|(key, _)| key == "code_challenge")
            .map(|(_, value)| value.into_owned())
            .unwrap()
    }

    /// Plays the browser: the provider redirects to the loopback callback.
    async fn browser_calls(url: String) -> Option<u16> {
        match reqwest::get(url).await {
            Ok(response) => Some(response.status().as_u16()),
            Err(_) => None,
        }
    }

    #[tokio::test]
    async fn a_matching_callback_is_exchanged_with_the_verifier() {
        let challenge = Arc::new(Mutex::new(String::new()));
        let (token_endpoint, seen) = fake_provider(challenge.clone()).await;
        let mut pending = start(&auth_base(), "google").await.unwrap();
        *challenge.lock().unwrap() = challenge_of(&pending.authorize_url);
        let (redirect, state) = redirect_and_state(&pending.authorize_url);
        assert_eq!(redirect.host_str(), Some("127.0.0.1"));

        let (_cancel, cancelled) = pending.cancel_handle();
        let browser = tokio::spawn(browser_calls(format!(
            "{redirect}?state={state}&code=good-code"
        )));
        let tokens = pending
            .finish(&token_endpoint, "anon", Duration::from_secs(10), cancelled)
            .await
            .unwrap();
        assert_eq!(tokens.access_token, "access-from-fake");
        assert_eq!(tokens.refresh_token.as_deref(), Some("refresh-from-fake"));
        assert_eq!(
            browser.await.unwrap(),
            Some(200),
            "the browser gets a closing page"
        );

        // The verifier went to the token endpoint only, and matched the challenge.
        let bodies = seen.0.lock().unwrap().clone();
        assert_eq!(bodies.len(), 1);
        assert_eq!(
            pkce::challenge_for(bodies[0]["code_verifier"].as_str().unwrap()),
            *challenge.lock().unwrap()
        );

        // The listener closed with the attempt: a replayed callback cannot connect.
        assert_eq!(
            browser_calls(format!("{redirect}?state={state}&code=good-code")).await,
            None
        );
    }

    #[tokio::test]
    async fn a_foreign_state_or_provider_error_never_reaches_the_token_endpoint() {
        let challenge = Arc::new(Mutex::new(String::new()));
        let (token_endpoint, seen) = fake_provider(challenge).await;

        let mut pending = start(&auth_base(), "google").await.unwrap();
        let (redirect, _) = redirect_and_state(&pending.authorize_url);
        let (_cancel, cancelled) = pending.cancel_handle();
        tokio::spawn(browser_calls(format!(
            "{redirect}?state=someone-else&code=good-code"
        )));
        let error = pending
            .finish(&token_endpoint, "anon", Duration::from_secs(10), cancelled)
            .await
            .unwrap_err();
        assert_eq!(error, LoginError::Callback(CallbackError::StateMismatch));

        let mut pending = start(&auth_base(), "google").await.unwrap();
        let (redirect, state) = redirect_and_state(&pending.authorize_url);
        let (_cancel, cancelled) = pending.cancel_handle();
        tokio::spawn(browser_calls(format!(
            "{redirect}?state={state}&error=access_denied"
        )));
        let error = pending
            .finish(&token_endpoint, "anon", Duration::from_secs(10), cancelled)
            .await
            .unwrap_err();
        assert_eq!(
            error,
            LoginError::Callback(CallbackError::ProviderError("access_denied".into()))
        );

        assert!(seen.0.lock().unwrap().is_empty(), "no code was exchanged");
    }

    #[tokio::test]
    async fn a_rejected_code_is_an_exchange_error() {
        let challenge = Arc::new(Mutex::new(String::new()));
        let (token_endpoint, _) = fake_provider(challenge.clone()).await;
        let mut pending = start(&auth_base(), "google").await.unwrap();
        *challenge.lock().unwrap() = challenge_of(&pending.authorize_url);
        let (redirect, state) = redirect_and_state(&pending.authorize_url);
        let (_cancel, cancelled) = pending.cancel_handle();
        tokio::spawn(browser_calls(format!(
            "{redirect}?state={state}&code=stale-code"
        )));
        let error = pending
            .finish(&token_endpoint, "anon", Duration::from_secs(10), cancelled)
            .await
            .unwrap_err();
        assert_eq!(
            error,
            LoginError::Exchange("token endpoint returned 400".into())
        );
    }

    #[tokio::test]
    async fn cancel_and_timeout_end_the_attempt_and_close_the_listener() {
        let challenge = Arc::new(Mutex::new(String::new()));
        let (token_endpoint, _) = fake_provider(challenge).await;

        let mut pending = start(&auth_base(), "google").await.unwrap();
        let (redirect, state) = redirect_and_state(&pending.authorize_url);
        let (cancel, cancelled) = pending.cancel_handle();
        let finishing = tokio::spawn(async move {
            pending
                .finish(&token_endpoint, "anon", Duration::from_secs(30), cancelled)
                .await
        });
        cancel.cancel();
        assert_eq!(finishing.await.unwrap().unwrap_err(), LoginError::Cancelled);
        assert_eq!(
            browser_calls(format!("{redirect}?state={state}&code=good-code")).await,
            None,
            "a late callback after cancel finds no listener"
        );

        let challenge = Arc::new(Mutex::new(String::new()));
        let (token_endpoint, _) = fake_provider(challenge).await;
        let mut pending = start(&auth_base(), "google").await.unwrap();
        let (_cancel, cancelled) = pending.cancel_handle();
        let error = pending
            .finish(
                &token_endpoint,
                "anon",
                Duration::from_millis(200),
                cancelled,
            )
            .await
            .unwrap_err();
        assert_eq!(error, LoginError::TimedOut);
    }

    #[tokio::test]
    async fn a_plain_http_token_endpoint_is_refused() {
        let mut pending = start(&auth_base(), "google").await.unwrap();
        let (redirect, state) = redirect_and_state(&pending.authorize_url);
        let (_cancel, cancelled) = pending.cancel_handle();
        tokio::spawn(browser_calls(format!(
            "{redirect}?state={state}&code=good-code"
        )));
        let error = pending
            .finish(
                &Url::parse("http://provider.example.test/auth/v1/token").unwrap(),
                "anon",
                Duration::from_secs(10),
                cancelled,
            )
            .await
            .unwrap_err();
        assert!(matches!(error, LoginError::Exchange(message) if message.contains("https")));
    }
}
