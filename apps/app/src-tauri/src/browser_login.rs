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
//! `run_browser_login` is what the `start_browser_login` command calls: it
//! opens the system browser and turns the provider's tokens into the session
//! the renderer stores. Registering this loopback redirect with the identity
//! provider is still an owner decision, so an attempt can only succeed once
//! that allowlist entry exists. Tests drive the same path against a fake
//! provider with a fake opener.

use std::time::Duration;

use serde::{Deserialize, Serialize};
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
    /// The attempt could not be built from the given identity base URL.
    Config(String),
    /// The system browser could not be opened.
    Browser(String),
}

/// Renderer-facing failure: a stable `code` the UI can branch on plus a
/// message safe to show. Neither ever carries the verifier or a token.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoginFailure {
    pub(crate) code: &'static str,
    pub(crate) message: String,
}

impl LoginFailure {
    /// A problem the command found before the attempt started.
    pub(crate) fn invalid_config(message: &str) -> Self {
        Self {
            code: "invalid_config",
            message: message.to_string(),
        }
    }

    /// Another attempt is already waiting for its callback.
    pub(crate) fn already_running() -> Self {
        Self {
            code: "already_running",
            message:
                "이미 브라우저 로그인이 진행 중입니다. 끝나거나 취소된 뒤에 다시 시도해 주세요."
                    .to_string(),
        }
    }
}

impl From<LoginError> for LoginFailure {
    fn from(error: LoginError) -> Self {
        let code = match &error {
            LoginError::TimedOut => "timeout",
            LoginError::Cancelled => "cancelled",
            LoginError::Callback(_) => "callback_rejected",
            LoginError::Exchange(_) => "exchange_failed",
            LoginError::Listener(_) => "listener_unavailable",
            LoginError::Config(_) => "invalid_config",
            LoginError::Browser(_) => "browser_unavailable",
        };
        // The internal reasons stay inside the process: only the status the
        // provider reported for this attempt (a short error name) is repeated.
        let message = match error {
            LoginError::TimedOut => {
                "브라우저 로그인 응답을 기다리다 시간이 초과되었습니다. 다시 시도해 주세요."
                    .to_string()
            }
            LoginError::Cancelled => "로그인을 취소했습니다.".to_string(),
            LoginError::Callback(CallbackError::WrongRedirect) => {
                "로그인 응답이 이 시도의 주소와 일치하지 않습니다.".to_string()
            }
            LoginError::Callback(CallbackError::StateMismatch) => {
                "로그인 응답의 state 값이 일치하지 않아 요청을 거부했습니다.".to_string()
            }
            LoginError::Callback(CallbackError::MissingCode) => {
                "로그인 응답에 인증 코드가 없습니다.".to_string()
            }
            LoginError::Callback(CallbackError::ProviderError(reason)) => {
                format!("로그인 제공자가 요청을 거부했습니다({reason}).")
            }
            LoginError::Exchange(_) => {
                "인증 코드를 토큰으로 바꾸지 못했습니다. 잠시 후 다시 시도해 주세요.".to_string()
            }
            LoginError::Listener(_) => {
                "로그인 대기 주소를 열지 못했습니다. 다시 시도해 주세요.".to_string()
            }
            LoginError::Config(_) => {
                "로그인 설정이 올바르지 않습니다. 앱 설정을 확인해 주세요.".to_string()
            }
            LoginError::Browser(_) => {
                "브라우저를 열지 못했습니다. 기본 브라우저를 확인해 주세요.".to_string()
            }
        };
        Self { code, message }
    }
}

/// Tokens the provider returned for the exchanged code.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct LoginTokens {
    pub(crate) access_token: String,
    #[serde(default)]
    pub(crate) refresh_token: Option<String>,
    #[serde(default)]
    pub(crate) expires_in: Option<u64>,
    #[serde(default)]
    pub(crate) user: Option<LoginUser>,
}

/// The public part of the provider's `user` object.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub(crate) struct LoginUser {
    #[serde(default)]
    pub(crate) id: Option<String>,
    #[serde(default)]
    pub(crate) email: Option<String>,
}

/// What the renderer stores as a session: the tokens plus the identity the
/// local drafts and the OS credential store key on.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BrowserLoginSession {
    pub(crate) access_token: String,
    pub(crate) refresh_token: Option<String>,
    pub(crate) expires_in_seconds: Option<u64>,
    pub(crate) user_id: String,
    pub(crate) email: Option<String>,
}

/// The credential store only accepts a session whose user id is the token's
/// own `sub`, so that subject wins; the provider's reported id is the fallback
/// for a token that is not a JWT.
fn session_from(tokens: LoginTokens) -> BrowserLoginSession {
    let LoginTokens {
        access_token,
        refresh_token,
        expires_in,
        user,
    } = tokens;
    let user_id = crate::token_subject(&access_token)
        .ok()
        .or_else(|| user.as_ref().and_then(|user| user.id.clone()))
        .unwrap_or_default();
    BrowserLoginSession {
        access_token,
        refresh_token,
        expires_in_seconds: expires_in,
        user_id,
        email: user.and_then(|user| user.email),
    }
}

/// A started attempt: open `authorize_url` in the browser, then `finish`.
pub(crate) struct PendingLogin {
    pub(crate) authorize_url: Url,
    redirect: Url,
    attempt: PkceAttempt,
    listener: TcpListener,
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
    let attempt = pkce::new_attempt().map_err(LoginError::Config)?;
    let authorize_url = pkce::authorize_url(auth_base, provider, &redirect, &attempt)
        .map_err(LoginError::Config)?;
    Ok(PendingLogin {
        authorize_url,
        redirect,
        attempt,
        listener,
    })
}

impl PendingLogin {
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

/// One complete attempt: bind the loopback listener, open the provider in the
/// system browser, wait for the single callback and exchange its code.
///
/// `open_browser` is injected so tests can drive the flow without a real
/// browser; in the app it opens the system browser, which is what makes this
/// the external-browser flow. `cancelled` ends the attempt (and closes the
/// listener) as soon as its sender fires. The token endpoint is derived from
/// `auth_base` (`.../auth/v1/` + `token`), so the caller only supplies the
/// public provider address.
pub(crate) async fn run_browser_login<F>(
    auth_base: &Url,
    provider: &str,
    api_key: &str,
    timeout: Duration,
    open_browser: F,
    cancelled: oneshot::Receiver<()>,
) -> Result<BrowserLoginSession, LoginError>
where
    F: FnOnce(&Url) -> Result<(), String>,
{
    let token_endpoint = auth_base
        .join("token")
        .map_err(|error| LoginError::Config(error.to_string()))?;
    let pending = start(auth_base, provider).await?;
    open_browser(&pending.authorize_url).map_err(LoginError::Browser)?;
    let tokens = pending
        .finish(&token_endpoint, api_key, timeout, cancelled)
        .await?;
    Ok(session_from(tokens))
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
                        "expires_in": 3600,
                        "user": { "id": "fake-user", "email": "teacher@example.test" }
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

    /// The provider base the command resolves `token` against. Pointing it at
    /// the fake provider's own loopback root lets the whole command path,
    /// including the token exchange, run against the fake.
    fn provider_base(token_endpoint: &Url) -> Url {
        let mut base = token_endpoint.clone();
        base.set_path("/auth/v1/");
        base.set_query(None);
        base
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
        let pending = start(&auth_base(), "google").await.unwrap();
        *challenge.lock().unwrap() = challenge_of(&pending.authorize_url);
        let (redirect, state) = redirect_and_state(&pending.authorize_url);
        assert_eq!(redirect.host_str(), Some("127.0.0.1"));

        let (_cancel, cancelled) = oneshot::channel();
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

        let pending = start(&auth_base(), "google").await.unwrap();
        let (redirect, _) = redirect_and_state(&pending.authorize_url);
        let (_cancel, cancelled) = oneshot::channel();
        tokio::spawn(browser_calls(format!(
            "{redirect}?state=someone-else&code=good-code"
        )));
        let error = pending
            .finish(&token_endpoint, "anon", Duration::from_secs(10), cancelled)
            .await
            .unwrap_err();
        assert_eq!(error, LoginError::Callback(CallbackError::StateMismatch));

        let pending = start(&auth_base(), "google").await.unwrap();
        let (redirect, state) = redirect_and_state(&pending.authorize_url);
        let (_cancel, cancelled) = oneshot::channel();
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
        let pending = start(&auth_base(), "google").await.unwrap();
        *challenge.lock().unwrap() = challenge_of(&pending.authorize_url);
        let (redirect, state) = redirect_and_state(&pending.authorize_url);
        let (_cancel, cancelled) = oneshot::channel();
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

        let pending = start(&auth_base(), "google").await.unwrap();
        let (redirect, state) = redirect_and_state(&pending.authorize_url);
        let (cancel, cancelled) = oneshot::channel();
        let finishing = tokio::spawn(async move {
            pending
                .finish(&token_endpoint, "anon", Duration::from_secs(30), cancelled)
                .await
        });
        let _ = cancel.send(());
        assert_eq!(finishing.await.unwrap().unwrap_err(), LoginError::Cancelled);
        assert_eq!(
            browser_calls(format!("{redirect}?state={state}&code=good-code")).await,
            None,
            "a late callback after cancel finds no listener"
        );

        let challenge = Arc::new(Mutex::new(String::new()));
        let (token_endpoint, _) = fake_provider(challenge).await;
        let pending = start(&auth_base(), "google").await.unwrap();
        let (_cancel, cancelled) = oneshot::channel();
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
        let pending = start(&auth_base(), "google").await.unwrap();
        let (redirect, state) = redirect_and_state(&pending.authorize_url);
        let (_cancel, cancelled) = oneshot::channel();
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

    /// Plays the browser for the command-layer function: remembers the
    /// authorize URL the app asked for, then sends the provider's redirect to
    /// the loopback callback.
    fn fake_opener(
        authorize_url: Arc<Mutex<Option<Url>>>,
        challenge: Arc<Mutex<String>>,
    ) -> impl FnOnce(&Url) -> Result<(), String> {
        move |authorize| {
            *authorize_url.lock().unwrap() = Some(authorize.clone());
            *challenge.lock().unwrap() = challenge_of(authorize);
            let (redirect, state) = redirect_and_state(authorize);
            tokio::spawn(browser_calls(format!(
                "{redirect}?state={state}&code=good-code"
            )));
            Ok(())
        }
    }

    #[tokio::test]
    async fn the_command_path_runs_authorize_callback_and_exchange() {
        let challenge = Arc::new(Mutex::new(String::new()));
        let (token_endpoint, seen) = fake_provider(challenge.clone()).await;
        let base = provider_base(&token_endpoint);
        let authorize_url = Arc::new(Mutex::new(None));

        let (_cancel, cancelled) = oneshot::channel();
        let session = run_browser_login(
            &base,
            "google",
            "anon",
            Duration::from_secs(10),
            fake_opener(authorize_url.clone(), challenge.clone()),
            cancelled,
        )
        .await
        .unwrap();

        // The opened URL is the provider's authorize endpoint, carries the
        // S256 challenge, and redirects only to a loopback port.
        let authorize = authorize_url.lock().unwrap().clone().unwrap();
        assert_eq!(authorize.path(), "/auth/v1/authorize");
        let pairs: Vec<(String, String)> = authorize.query_pairs().into_owned().collect();
        assert!(pairs.contains(&("provider".to_string(), "google".to_string())));
        assert!(pairs.contains(&("code_challenge_method".to_string(), "s256".to_string())));
        let (redirect, _) = redirect_and_state(&authorize);
        assert_eq!(redirect.host_str(), Some("127.0.0.1"));
        // The verifier never rides on the URL the browser opens.
        assert!(
            !authorize
                .query_pairs()
                .any(|(key, _)| key == "code_verifier")
        );

        // The code was exchanged once, with the verifier that matches the
        // challenge the authorize URL carried.
        let bodies = seen.0.lock().unwrap().clone();
        assert_eq!(bodies.len(), 1);
        assert_eq!(bodies[0]["auth_code"], "good-code");
        assert_eq!(
            pkce::challenge_for(bodies[0]["code_verifier"].as_str().unwrap()),
            *challenge.lock().unwrap()
        );

        // The renderer receives the session fields it stores.
        assert_eq!(session.access_token, "access-from-fake");
        assert_eq!(session.refresh_token.as_deref(), Some("refresh-from-fake"));
        assert_eq!(session.expires_in_seconds, Some(3600));
        assert_eq!(session.user_id, "fake-user");
        assert_eq!(session.email.as_deref(), Some("teacher@example.test"));
    }

    #[tokio::test]
    async fn cancel_through_the_command_path_closes_the_listener() {
        let (token_endpoint, _) = fake_provider(Arc::new(Mutex::new(String::new()))).await;
        let base = provider_base(&token_endpoint);
        let (send_redirect, redirect_seen) = oneshot::channel();
        let (cancel, cancelled) = oneshot::channel();

        let running = tokio::spawn(async move {
            run_browser_login(
                &base,
                "google",
                "anon",
                Duration::from_secs(30),
                move |authorize: &Url| {
                    let (redirect, _) = redirect_and_state(authorize);
                    let _ = send_redirect.send(redirect);
                    Ok(())
                },
                cancelled,
            )
            .await
        });

        let redirect = redirect_seen.await.unwrap();
        let _ = cancel.send(());
        assert_eq!(running.await.unwrap().unwrap_err(), LoginError::Cancelled);
        assert_eq!(
            browser_calls(format!("{redirect}?state=late&code=good-code")).await,
            None,
            "cancelling closes the loopback listener"
        );
    }

    #[tokio::test]
    async fn timeout_through_the_command_path_closes_the_listener() {
        let (token_endpoint, seen) = fake_provider(Arc::new(Mutex::new(String::new()))).await;
        let base = provider_base(&token_endpoint);
        let (send_redirect, redirect_seen) = oneshot::channel();

        let (_cancel, cancelled) = oneshot::channel();
        let error = run_browser_login(
            &base,
            "google",
            "anon",
            Duration::from_millis(150),
            move |authorize: &Url| {
                let (redirect, _) = redirect_and_state(authorize);
                let _ = send_redirect.send(redirect);
                Ok(())
            },
            cancelled,
        )
        .await
        .unwrap_err();

        assert_eq!(error, LoginError::TimedOut);
        let redirect = redirect_seen.await.unwrap();
        assert_eq!(
            browser_calls(format!("{redirect}?state=late&code=good-code")).await,
            None,
            "a callback after the deadline finds no listener"
        );
        assert!(seen.0.lock().unwrap().is_empty(), "no code was exchanged");
    }

    /// Unsigned JWT-shaped token carrying only a subject, as the identity
    /// provider's access token shape is.
    fn jwt_with_subject(subject: &str) -> String {
        use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
        let claims = URL_SAFE_NO_PAD.encode(format!(r#"{{"sub":"{subject}"}}"#));
        format!("{header}.{claims}.signature")
    }

    #[test]
    fn the_session_owner_is_the_token_subject() {
        let session = session_from(LoginTokens {
            access_token: jwt_with_subject("user-9"),
            refresh_token: Some("refresh".to_string()),
            expires_in: Some(60),
            user: Some(LoginUser {
                id: Some("different-id".to_string()),
                email: Some("teacher@example.test".to_string()),
            }),
        });
        assert_eq!(session.user_id, "user-9");
        assert_eq!(session.email.as_deref(), Some("teacher@example.test"));
        assert_eq!(session.expires_in_seconds, Some(60));

        // A token the app cannot read falls back to the provider's own id.
        let opaque = session_from(LoginTokens {
            access_token: "opaque-token".to_string(),
            refresh_token: None,
            expires_in: None,
            user: Some(LoginUser {
                id: Some("user-1".to_string()),
                email: None,
            }),
        });
        assert_eq!(opaque.user_id, "user-1");
        assert_eq!(opaque.expires_in_seconds, None);
    }

    #[test]
    fn failures_are_a_stable_code_and_a_safe_message() {
        for (error, code) in [
            (LoginError::TimedOut, "timeout"),
            (LoginError::Cancelled, "cancelled"),
            (
                LoginError::Callback(CallbackError::StateMismatch),
                "callback_rejected",
            ),
            (
                LoginError::Exchange("secret".to_string()),
                "exchange_failed",
            ),
            (
                LoginError::Listener("socket detail".to_string()),
                "listener_unavailable",
            ),
            (
                LoginError::Config("base detail".to_string()),
                "invalid_config",
            ),
            (
                LoginError::Browser("opener detail".to_string()),
                "browser_unavailable",
            ),
        ] {
            let failure = LoginFailure::from(error);
            assert_eq!(failure.code, code);
            assert!(!failure.message.trim().is_empty());
            // Internal reasons never reach the renderer as-is.
            assert!(!failure.message.contains("detail"));
        }

        assert_eq!(
            LoginFailure::from(LoginError::Cancelled).message,
            "로그인을 취소했습니다."
        );
        assert!(
            LoginFailure::from(LoginError::Callback(CallbackError::ProviderError(
                "access_denied".to_string()
            )))
            .message
            .contains("access_denied")
        );
        assert_eq!(LoginFailure::already_running().code, "already_running");
        assert_eq!(
            LoginFailure::invalid_config("문제가 있습니다.").code,
            "invalid_config"
        );
    }
}
