//! External-browser login (S-05) building blocks: PKCE verifier/challenge
//! (RFC 7636, S256), an unguessable `state`, and strict callback validation.
//!
//! The desktop app will open the identity provider in the system browser and
//! receive the result on a loopback redirect. This module holds the security
//! decisions for that exchange so they are tested before the flow is wired:
//! the redirect allowlist entry is an owner decision on the identity provider,
//! so no UI calls this yet.

use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};
use url::Url;

/// 32 random bytes → 43 base64url characters, inside RFC 7636's 43..=128.
const RANDOM_BYTES: usize = 32;
const MAX_CODE_LEN: usize = 512;

/// One login attempt. Keep it in memory only for the duration of the attempt;
/// the verifier must never be logged, persisted, or sent anywhere except the
/// token exchange.
pub(crate) struct PkceAttempt {
    pub(crate) verifier: String,
    pub(crate) challenge: String,
    pub(crate) state: String,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CallbackError {
    /// The callback did not arrive on the redirect this attempt registered.
    WrongRedirect,
    /// The provider reported an error (user cancelled, access denied, ...).
    ProviderError(String),
    /// `state` missing or different: possible login CSRF, reject.
    StateMismatch,
    /// No usable authorization code.
    MissingCode,
}

fn random_token() -> Result<String, String> {
    let mut bytes = [0_u8; RANDOM_BYTES];
    getrandom::getrandom(&mut bytes)
        .map_err(|error| format!("secure random unavailable: {error}"))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

/// S256 challenge: BASE64URL(SHA256(ASCII(verifier))).
pub(crate) fn challenge_for(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

pub(crate) fn new_attempt() -> Result<PkceAttempt, String> {
    let verifier = random_token()?;
    let state = random_token()?;
    let challenge = challenge_for(&verifier);
    Ok(PkceAttempt {
        verifier,
        challenge,
        state,
    })
}

/// Loopback redirects only: the system browser hands the result back to this
/// machine, never to a remote host.
pub(crate) fn is_loopback_redirect(redirect: &Url) -> bool {
    redirect.scheme() == "http"
        && matches!(redirect.host_str(), Some("127.0.0.1") | Some("[::1]"))
        && redirect.port().is_some()
        && redirect.query().is_none()
        && redirect.fragment().is_none()
}

/// Supabase Auth authorize URL for the PKCE flow. Our own `state` rides on the
/// redirect so the callback can be matched to this attempt.
pub(crate) fn authorize_url(
    auth_base: &Url,
    provider: &str,
    redirect: &Url,
    attempt: &PkceAttempt,
) -> Result<Url, String> {
    if !is_loopback_redirect(redirect) {
        return Err("redirect must be an http loopback address with a port".into());
    }
    if provider.is_empty()
        || !provider
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("invalid provider name".into());
    }
    let mut redirect_with_state = redirect.clone();
    redirect_with_state
        .query_pairs_mut()
        .append_pair("state", &attempt.state);
    let mut url = auth_base
        .join("authorize")
        .map_err(|error| format!("invalid auth base URL: {error}"))?;
    url.query_pairs_mut()
        .append_pair("provider", provider)
        .append_pair("redirect_to", redirect_with_state.as_str())
        .append_pair("code_challenge", &attempt.challenge)
        .append_pair("code_challenge_method", "s256");
    Ok(url)
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

/// Validates the loopback callback and returns the authorization code.
/// Order matters: the redirect is checked first, then `state` (in constant
/// time), then provider errors, and only then the code is accepted.
pub(crate) fn validate_callback(
    callback: &Url,
    expected_redirect: &Url,
    expected_state: &str,
) -> Result<String, CallbackError> {
    let same_endpoint = callback.scheme() == expected_redirect.scheme()
        && callback.host_str() == expected_redirect.host_str()
        && callback.port() == expected_redirect.port()
        && callback.path() == expected_redirect.path();
    if !same_endpoint {
        return Err(CallbackError::WrongRedirect);
    }

    let mut state = None;
    let mut code = None;
    let mut error = None;
    for (key, value) in callback.query_pairs() {
        let slot = match key.as_ref() {
            "state" => &mut state,
            "code" => &mut code,
            "error" => &mut error,
            _ => continue,
        };
        if slot.is_some() {
            // A repeated parameter is ambiguous; treat it as tampering.
            return Err(CallbackError::StateMismatch);
        }
        *slot = Some(value.into_owned());
    }

    match state {
        Some(state)
            if !expected_state.is_empty()
                && constant_time_eq(state.as_bytes(), expected_state.as_bytes()) => {}
        _ => return Err(CallbackError::StateMismatch),
    }
    // Only after `state` matches: an error response is bound to this attempt
    // too, so a stray request cannot cancel someone's login.
    if let Some(error) = error {
        return Err(CallbackError::ProviderError(
            error.chars().take(100).collect(),
        ));
    }
    match code {
        Some(code)
            if !code.is_empty()
                && code.len() <= MAX_CODE_LEN
                && code
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~')) =>
        {
            Ok(code)
        }
        _ => Err(CallbackError::MissingCode),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn redirect() -> Url {
        Url::parse("http://127.0.0.1:53682/callback").unwrap()
    }

    #[test]
    fn challenge_matches_rfc7636_appendix_b() {
        assert_eq!(
            challenge_for("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn attempts_are_random_and_rfc_sized() {
        let first = new_attempt().unwrap();
        let second = new_attempt().unwrap();
        assert_eq!(first.verifier.len(), 43);
        assert_eq!(first.state.len(), 43);
        assert_ne!(first.verifier, second.verifier);
        assert_ne!(first.state, second.state);
        assert_ne!(first.verifier, first.state);
        assert_eq!(first.challenge, challenge_for(&first.verifier));
        assert!(
            first
                .verifier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }

    #[test]
    fn authorize_url_carries_challenge_and_state_but_never_the_verifier() {
        let attempt = new_attempt().unwrap();
        let base = Url::parse("https://project.supabase.co/auth/v1/").unwrap();
        let url = authorize_url(&base, "google", &redirect(), &attempt).unwrap();
        assert_eq!(url.path(), "/auth/v1/authorize");
        let pairs: Vec<(String, String)> = url.query_pairs().into_owned().collect();
        assert!(pairs.contains(&("code_challenge".into(), attempt.challenge.clone())));
        assert!(pairs.contains(&("code_challenge_method".into(), "s256".into())));
        let redirect_to = &pairs.iter().find(|(k, _)| k == "redirect_to").unwrap().1;
        assert!(redirect_to.starts_with("http://127.0.0.1:53682/callback?state="));
        assert!(!url.as_str().contains(&attempt.verifier));
    }

    #[test]
    fn authorize_url_rejects_non_loopback_redirects_and_odd_providers() {
        let attempt = new_attempt().unwrap();
        let base = Url::parse("https://project.supabase.co/auth/v1/").unwrap();
        for bad in [
            "https://127.0.0.1:53682/callback",
            "http://example.com:53682/callback",
            "http://127.0.0.1/callback",
            "http://127.0.0.1:53682/callback?x=1",
        ] {
            let bad = Url::parse(bad).unwrap();
            assert!(
                authorize_url(&base, "google", &bad, &attempt).is_err(),
                "{bad}"
            );
        }
        assert!(authorize_url(&base, "goo gle", &redirect(), &attempt).is_err());
        assert!(authorize_url(&base, "", &redirect(), &attempt).is_err());
    }

    #[test]
    fn callback_with_matching_state_returns_code() {
        let callback =
            Url::parse("http://127.0.0.1:53682/callback?state=abc123&code=auth-code_1").unwrap();
        assert_eq!(
            validate_callback(&callback, &redirect(), "abc123"),
            Ok("auth-code_1".into())
        );
    }

    #[test]
    fn callback_rejects_state_problems() {
        for query in [
            "code=c",
            "state=other&code=c",
            "state=abc123x&code=c",
            "state=abc123&state=abc123&code=c",
        ] {
            let callback = Url::parse(&format!("http://127.0.0.1:53682/callback?{query}")).unwrap();
            assert_eq!(
                validate_callback(&callback, &redirect(), "abc123"),
                Err(CallbackError::StateMismatch),
                "{query}"
            );
        }
        let callback = Url::parse("http://127.0.0.1:53682/callback?state=&code=c").unwrap();
        assert_eq!(
            validate_callback(&callback, &redirect(), ""),
            Err(CallbackError::StateMismatch)
        );
    }

    #[test]
    fn callback_rejects_wrong_endpoint_provider_errors_and_bad_codes() {
        let other_port = Url::parse("http://127.0.0.1:1/callback?state=abc123&code=c").unwrap();
        assert_eq!(
            validate_callback(&other_port, &redirect(), "abc123"),
            Err(CallbackError::WrongRedirect)
        );
        let other_path = Url::parse("http://127.0.0.1:53682/other?state=abc123&code=c").unwrap();
        assert_eq!(
            validate_callback(&other_path, &redirect(), "abc123"),
            Err(CallbackError::WrongRedirect)
        );

        let denied =
            Url::parse("http://127.0.0.1:53682/callback?state=abc123&error=access_denied").unwrap();
        assert_eq!(
            validate_callback(&denied, &redirect(), "abc123"),
            Err(CallbackError::ProviderError("access_denied".into()))
        );
        // An error without the attempt's state is a stray request, not the
        // provider answering this login.
        for query in ["error=access_denied", "state=other&error=access_denied"] {
            let stray = Url::parse(&format!("http://127.0.0.1:53682/callback?{query}")).unwrap();
            assert_eq!(
                validate_callback(&stray, &redirect(), "abc123"),
                Err(CallbackError::StateMismatch),
                "{query}"
            );
        }

        for query in [
            "state=abc123",
            "state=abc123&code=",
            "state=abc123&code=a%20b",
            "state=abc123&code=a%3Cb",
        ] {
            let callback = Url::parse(&format!("http://127.0.0.1:53682/callback?{query}")).unwrap();
            assert_eq!(
                validate_callback(&callback, &redirect(), "abc123"),
                Err(CallbackError::MissingCode),
                "{query}"
            );
        }
        let long = format!(
            "http://127.0.0.1:53682/callback?state=abc123&code={}",
            "a".repeat(MAX_CODE_LEN + 1)
        );
        assert_eq!(
            validate_callback(&Url::parse(&long).unwrap(), &redirect(), "abc123"),
            Err(CallbackError::MissingCode)
        );
    }
}
