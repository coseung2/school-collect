use std::{future::Future, pin::Pin, sync::Arc};

use anyhow::{Context, bail};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header, jwk::JwkSet};
use reqwest::Client;
use school_collect_domain::{TenantId, UserId};
use serde::Deserialize;
use tokio::sync::RwLock;
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Actor {
    pub user_id: UserId,
    pub tenant_id: TenantId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcConfig {
    pub issuer_url: Url,
    pub audience: String,
    /// Explicit JWKS location for issuers that do not advertise discovery.
    pub jwks_url: Option<Url>,
}

impl OidcConfig {
    pub fn from_env<F>(mut get: F) -> anyhow::Result<Self>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let issuer = get("OIDC_ISSUER_URL")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("OIDC_ISSUER_URL is required"))?;
        let issuer_url = Url::parse(&issuer)
            .map_err(|error| anyhow::anyhow!("OIDC_ISSUER_URL is invalid: {error}"))?;
        if issuer_url.scheme() != "https" {
            bail!("OIDC_ISSUER_URL must use https");
        }

        let audience = get("OIDC_AUDIENCE")
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("OIDC_AUDIENCE is required"))?;

        let jwks_url = match get("OIDC_JWKS_URL").filter(|value| !value.trim().is_empty()) {
            Some(value) => {
                let url = Url::parse(&value)
                    .map_err(|error| anyhow::anyhow!("OIDC_JWKS_URL is invalid: {error}"))?;
                if url.scheme() != "https" {
                    bail!("OIDC_JWKS_URL must use https");
                }
                Some(url)
            }
            None => None,
        };

        Ok(Self {
            issuer_url,
            audience,
            jwks_url,
        })
    }

    /// Discovery document location for this issuer.
    ///
    /// `Url::join` would replace the final path segment of an issuer such as
    /// `https://host/auth/v1`, so the discovery path is appended explicitly.
    pub fn discovery_url(&self) -> anyhow::Result<Url> {
        let base = self.issuer_url.as_str().trim_end_matches('/');
        Url::parse(&format!("{base}/.well-known/openid-configuration"))
            .context("OIDC discovery URL is not a valid URL")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMode {
    DisabledForDevelopment,
    Oidc,
}

impl AuthMode {
    pub fn from_env<F>(mut get: F) -> anyhow::Result<Self>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let environment = get("APP_ENV").unwrap_or_else(|| "development".to_owned());
        match get("APP_AUTH_MODE").as_deref() {
            Some("oidc") => Ok(Self::Oidc),
            Some("disabled") if environment == "development" => Ok(Self::DisabledForDevelopment),
            Some(value) => bail!(
                "APP_AUTH_MODE={value} is invalid for APP_ENV={environment}; only oidc is allowed outside development"
            ),
            None if environment == "development" => Ok(Self::DisabledForDevelopment),
            None => bail!("APP_AUTH_MODE=oidc is required outside development"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedPrincipal {
    pub issuer: String,
    pub subject: String,
    pub email: Option<String>,
}

pub fn extract_bearer_token(value: &str) -> anyhow::Result<&str> {
    let mut parts = value.split_ascii_whitespace();
    let scheme = parts.next().unwrap_or_default();
    let token = parts.next().unwrap_or_default();
    if !scheme.eq_ignore_ascii_case("bearer") || token.is_empty() || parts.next().is_some() {
        bail!("authorization header must contain one bearer token");
    }
    Ok(token)
}

#[derive(Debug, Deserialize)]
struct DiscoveryDocument {
    jwks_uri: String,
}

/// Source of the issuer's signing keys.
///
/// The default source fetches over HTTPS (discovery document, then JWKS). Tests
/// inject a source so key rotation, cache reuse, and provider outages can be
/// verified with real signed tokens and no network access.
pub trait KeySource: Send + Sync {
    fn fetch<'a>(
        &'a self,
        config: &'a OidcConfig,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<JwkSet>> + Send + 'a>>;
}

pub struct HttpKeySource {
    client: Client,
}

impl HttpKeySource {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub fn with_default_client() -> anyhow::Result<Self> {
        let client = Client::builder()
            .https_only(true)
            .user_agent(concat!("school-collect-api/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("failed to build OIDC HTTP client")?;
        Ok(Self::new(client))
    }
}

impl KeySource for HttpKeySource {
    fn fetch<'a>(
        &'a self,
        config: &'a OidcConfig,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<JwkSet>> + Send + 'a>> {
        Box::pin(async move {
            let jwks_url = match &config.jwks_url {
                Some(url) => url.clone(),
                None => {
                    let discovery_url = config
                        .discovery_url()
                        .context("invalid OIDC discovery URL")?;
                    let discovery = self
                        .client
                        .get(discovery_url)
                        .send()
                        .await
                        .context("OIDC discovery request failed")?
                        .error_for_status()
                        .context("OIDC discovery returned an error")?
                        .json::<DiscoveryDocument>()
                        .await
                        .context("invalid OIDC discovery response")?;
                    let url = Url::parse(&discovery.jwks_uri).context("invalid OIDC JWKS URL")?;
                    if url.scheme() != "https" {
                        bail!("OIDC JWKS URL must use https");
                    }
                    url
                }
            };

            let keys = self
                .client
                .get(jwks_url)
                .send()
                .await
                .context("OIDC JWKS request failed")?
                .error_for_status()
                .context("OIDC JWKS returned an error")?
                .json::<JwkSet>()
                .await
                .context("invalid OIDC JWKS response")?;

            Ok(keys)
        })
    }
}

#[derive(Debug, Deserialize)]
struct Claims {
    iss: String,
    sub: String,
    #[serde(default)]
    email: Option<String>,
}

/// Access-token signing algorithms this API is willing to accept.
///
/// Supabase Auth publishes an ES256 P-256 key by default and can publish RS256
/// keys on older projects, so both are accepted. Symmetric algorithms are
/// deliberately excluded: a shared secret must never be the verification path
/// for a public client.
const ALLOWED_ALGORITHMS: [Algorithm; 2] = [Algorithm::ES256, Algorithm::RS256];

pub struct OidcVerifier {
    config: OidcConfig,
    source: Arc<dyn KeySource>,
    keys: RwLock<Option<JwkSet>>,
}

impl OidcVerifier {
    pub fn new(config: OidcConfig) -> anyhow::Result<Arc<Self>> {
        Ok(Self::with_source(
            config,
            Arc::new(HttpKeySource::with_default_client()?),
        ))
    }

    /// Builds a verifier with an injected key source. Used by tests to exercise
    /// rotation and outage behavior without network access.
    pub fn with_source(config: OidcConfig, source: Arc<dyn KeySource>) -> Arc<Self> {
        Arc::new(Self {
            config,
            source,
            keys: RwLock::new(None),
        })
    }

    pub async fn verify(&self, authorization: &str) -> anyhow::Result<VerifiedPrincipal> {
        let token = extract_bearer_token(authorization)?;
        let header = decode_header(token).context("invalid JWT header")?;
        if !ALLOWED_ALGORITHMS.contains(&header.alg) {
            bail!("unsupported access token algorithm");
        }
        let key_id = header
            .kid
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("JWT kid is required"))?;

        let mut keys = self.load_keys(false).await?;
        let mut jwk = keys.find(key_id).cloned();
        if jwk.is_none() {
            // The provider may have rotated signing keys since the last fetch.
            keys = self.load_keys(true).await?;
            jwk = keys.find(key_id).cloned();
        }
        let jwk = jwk.ok_or_else(|| anyhow::anyhow!("JWT signing key was not found"))?;
        let decoding_key = DecodingKey::from_jwk(&jwk).context("invalid JWT signing key")?;

        let mut validation = Validation::new(header.alg);
        validation.set_issuer(&[self.config.issuer_url.as_str()]);
        validation.set_audience(&[self.config.audience.as_str()]);
        let claims = decode::<Claims>(token, &decoding_key, &validation)
            .context("JWT verification failed")?
            .claims;

        if claims.iss.trim_end_matches('/') != self.config.issuer_url.as_str().trim_end_matches('/')
        {
            bail!("JWT issuer does not match configured issuer");
        }
        if claims.sub.trim().is_empty() {
            bail!("JWT subject is required");
        }

        Ok(VerifiedPrincipal {
            issuer: claims.iss.trim_end_matches('/').to_owned(),
            subject: claims.sub,
            email: claims.email,
        })
    }

    /// Fetches the signing keys, optionally forcing a refresh after a `kid` miss.
    async fn load_keys(&self, force_refresh: bool) -> anyhow::Result<JwkSet> {
        if !force_refresh && let Some(keys) = self.keys.read().await.clone() {
            return Ok(keys);
        }

        let keys = self.source.fetch(&self.config).await?;
        if keys.keys.is_empty() {
            bail!("OIDC JWKS contained no keys");
        }

        *self.keys.write().await = Some(keys.clone());
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::{AuthMode, KeySource, OidcConfig, OidcVerifier, extract_bearer_token};
    use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
    use jsonwebtoken::{
        Algorithm, EncodingKey, Header,
        jwk::{Jwk, JwkSet},
    };
    use p256::{
        ecdsa::SigningKey,
        pkcs8::{EncodePrivateKey, LineEnding},
    };
    use rand_core::OsRng;
    use serde_json::json;
    use std::{
        collections::HashMap,
        future::Future,
        pin::Pin,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
        time::{SystemTime, UNIX_EPOCH},
    };

    /// Test-only signing key. Generated per test run; never stored on disk.
    struct TestKey {
        kid: String,
        encoding: EncodingKey,
        jwk: Jwk,
    }

    impl TestKey {
        fn generate(kid: &str) -> Self {
            let signing_key = SigningKey::random(&mut OsRng);
            let pem = signing_key.to_pkcs8_pem(LineEnding::LF).expect("pkcs8 pem");
            let encoding = EncodingKey::from_ec_pem(pem.as_bytes()).expect("ec encoding key");
            let point = signing_key.verifying_key().to_encoded_point(false);
            let jwk: Jwk = serde_json::from_value(json!({
                "kty": "EC",
                "crv": "P-256",
                "x": URL_SAFE_NO_PAD.encode(point.x().expect("x coordinate")),
                "y": URL_SAFE_NO_PAD.encode(point.y().expect("y coordinate")),
                "alg": "ES256",
                "use": "sig",
                "kid": kid,
            }))
            .expect("jwk");
            Self {
                kid: kid.to_owned(),
                encoding,
                jwk,
            }
        }

        fn key_set(&self) -> JwkSet {
            JwkSet {
                keys: vec![self.jwk.clone()],
            }
        }

        fn sign(&self, issuer: &str, audience: &str, subject: &str) -> String {
            let expires = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock")
                .as_secs()
                + 600;
            let header = Header {
                alg: Algorithm::ES256,
                kid: Some(self.kid.clone()),
                ..Header::default()
            };
            jsonwebtoken::encode(
                &header,
                &json!({
                    "iss": issuer,
                    "sub": subject,
                    "aud": audience,
                    "exp": expires,
                }),
                &self.encoding,
            )
            .expect("signed token")
        }
    }

    /// Key source with scripted responses and a fetch counter.
    struct FakeKeySource {
        responses: Mutex<Vec<anyhow::Result<JwkSet>>>,
        fetches: AtomicUsize,
    }

    impl FakeKeySource {
        fn new(responses: Vec<anyhow::Result<JwkSet>>) -> Arc<Self> {
            Arc::new(Self {
                responses: Mutex::new(responses),
                fetches: AtomicUsize::new(0),
            })
        }

        fn fetch_count(&self) -> usize {
            self.fetches.load(Ordering::SeqCst)
        }
    }

    impl KeySource for FakeKeySource {
        fn fetch<'a>(
            &'a self,
            _config: &'a OidcConfig,
        ) -> Pin<Box<dyn Future<Output = anyhow::Result<JwkSet>> + Send + 'a>> {
            Box::pin(async move {
                self.fetches.fetch_add(1, Ordering::SeqCst);
                let mut responses = self.responses.lock().expect("responses");
                if responses.is_empty() {
                    anyhow::bail!("no scripted key set left");
                }
                if responses.len() == 1 {
                    // The last scripted response answers every further fetch.
                    return match &responses[0] {
                        Ok(keys) => Ok(keys.clone()),
                        Err(error) => Err(anyhow::anyhow!("{error}")),
                    };
                }
                responses.remove(0)
            })
        }
    }

    fn test_config() -> OidcConfig {
        let values = HashMap::from([
            ("OIDC_ISSUER_URL", "https://id.example.test/auth/v1"),
            ("OIDC_AUDIENCE", "authenticated"),
        ]);
        OidcConfig::from_env(|key| values.get(key).map(|value| (*value).to_owned())).unwrap()
    }

    #[test]
    fn oidc_config_requires_https_issuer_and_audience() {
        let values = HashMap::from([
            ("OIDC_ISSUER_URL", "https://id.example.test"),
            ("OIDC_AUDIENCE", "authenticated"),
        ]);

        let config = OidcConfig::from_env(|key| values.get(key).map(|value| (*value).to_owned()))
            .expect("valid OIDC configuration");

        assert_eq!(config.audience, "authenticated");
        assert_eq!(config.issuer_url.as_str(), "https://id.example.test/");
        assert!(config.jwks_url.is_none());
    }

    #[test]
    fn oidc_config_rejects_non_https_issuer() {
        let values = HashMap::from([
            ("OIDC_ISSUER_URL", "http://id.example.test"),
            ("OIDC_AUDIENCE", "authenticated"),
        ]);

        assert!(
            OidcConfig::from_env(|key| values.get(key).map(|value| (*value).to_owned())).is_err()
        );
    }

    #[test]
    fn oidc_config_accepts_an_explicit_https_jwks_url() {
        let values = HashMap::from([
            ("OIDC_ISSUER_URL", "https://id.example.test"),
            ("OIDC_AUDIENCE", "authenticated"),
            ("OIDC_JWKS_URL", "https://id.example.test/keys"),
        ]);

        let config = OidcConfig::from_env(|key| values.get(key).map(|value| (*value).to_owned()))
            .expect("valid OIDC configuration");

        assert_eq!(
            config.jwks_url.map(|url| url.to_string()),
            Some("https://id.example.test/keys".to_owned())
        );
    }

    #[test]
    fn development_auth_can_be_explicitly_disabled() {
        let values = HashMap::from([("APP_ENV", "development"), ("APP_AUTH_MODE", "disabled")]);
        assert_eq!(
            AuthMode::from_env(|key| values.get(key).map(|value| (*value).to_owned())).unwrap(),
            AuthMode::DisabledForDevelopment
        );
    }

    #[test]
    fn non_development_requires_oidc_mode() {
        let values = HashMap::from([(String::from("APP_ENV"), String::from("production"))]);
        assert!(AuthMode::from_env(|key| values.get(key).cloned()).is_err());
    }

    #[test]
    fn bearer_parser_rejects_ambiguous_headers() {
        assert_eq!(extract_bearer_token("Bearer abc").unwrap(), "abc");
        assert_eq!(extract_bearer_token("bearer abc").unwrap(), "abc");
        assert!(extract_bearer_token("Basic abc").is_err());
        assert!(extract_bearer_token("Bearer abc extra").is_err());
    }

    #[test]
    fn discovery_url_keeps_the_whole_issuer_path() {
        let values = HashMap::from([
            ("OIDC_ISSUER_URL", "https://id.example.test/auth/v1"),
            ("OIDC_AUDIENCE", "authenticated"),
        ]);
        let config =
            OidcConfig::from_env(|key| values.get(key).map(|value| (*value).to_owned())).unwrap();

        assert_eq!(
            config.discovery_url().unwrap().as_str(),
            "https://id.example.test/auth/v1/.well-known/openid-configuration"
        );
    }

    #[tokio::test]
    async fn verify_uses_cached_keys_until_a_kid_miss() {
        let key = TestKey::generate("key-1");
        let source = FakeKeySource::new(vec![Ok(key.key_set())]);
        let verifier = OidcVerifier::with_source(test_config(), source.clone());
        let token = key.sign("https://id.example.test/auth/v1", "authenticated", "user-1");

        let principal = verifier
            .verify(&format!("Bearer {token}"))
            .await
            .expect("first verification");
        assert_eq!(principal.subject, "user-1");
        assert_eq!(principal.issuer, "https://id.example.test/auth/v1");
        assert_eq!(source.fetch_count(), 1);

        verifier
            .verify(&format!("Bearer {token}"))
            .await
            .expect("second verification");
        assert_eq!(source.fetch_count(), 1, "cached keys must not refetch");
    }

    #[tokio::test]
    async fn verify_refreshes_keys_when_the_provider_rotates() {
        let old_key = TestKey::generate("key-old");
        let new_key = TestKey::generate("key-new");
        let source = FakeKeySource::new(vec![Ok(old_key.key_set()), Ok(new_key.key_set())]);
        let verifier = OidcVerifier::with_source(test_config(), source.clone());
        let rotated = new_key.sign("https://id.example.test/auth/v1", "authenticated", "user-2");

        let principal = verifier
            .verify(&format!("Bearer {rotated}"))
            .await
            .expect("rotated key verification");
        assert_eq!(principal.subject, "user-2");
        assert_eq!(source.fetch_count(), 2, "one fetch, then one refresh");

        verifier
            .verify(&format!("Bearer {rotated}"))
            .await
            .expect("cached rotated key verification");
        assert_eq!(source.fetch_count(), 2);
    }

    #[tokio::test]
    async fn verify_fails_when_the_kid_stays_unknown() {
        let known = TestKey::generate("key-1");
        let unknown = TestKey::generate("key-2");
        let source = FakeKeySource::new(vec![Ok(known.key_set())]);
        let verifier = OidcVerifier::with_source(test_config(), source.clone());
        let token = unknown.sign("https://id.example.test/auth/v1", "authenticated", "user-3");

        let error = verifier
            .verify(&format!("Bearer {token}"))
            .await
            .expect_err("unknown kid must fail");

        assert!(error.to_string().contains("signing key"));
        assert_eq!(source.fetch_count(), 2, "unknown kid forces one refresh");
    }

    #[tokio::test]
    async fn provider_outage_does_not_discard_cached_keys() {
        let key = TestKey::generate("key-1");
        let unknown = TestKey::generate("key-2");
        let source = FakeKeySource::new(vec![
            Ok(key.key_set()),
            Err(anyhow::anyhow!("JWKS endpoint is unreachable")),
        ]);
        let verifier = OidcVerifier::with_source(test_config(), source.clone());
        let known_token = key.sign("https://id.example.test/auth/v1", "authenticated", "user-4");
        let unknown_token =
            unknown.sign("https://id.example.test/auth/v1", "authenticated", "user-5");

        verifier
            .verify(&format!("Bearer {known_token}"))
            .await
            .expect("cached verification");
        assert_eq!(source.fetch_count(), 1);

        // A kid miss during an outage fails, but the cached keys stay usable.
        let error = verifier
            .verify(&format!("Bearer {unknown_token}"))
            .await
            .expect_err("outage must fail the refresh");
        assert!(error.to_string().contains("unreachable"));

        verifier
            .verify(&format!("Bearer {known_token}"))
            .await
            .expect("cached keys survive the outage");
        assert_eq!(source.fetch_count(), 2);
    }

    #[tokio::test]
    async fn verify_rejects_tokens_with_wrong_claims() {
        let key = TestKey::generate("key-1");
        let source = FakeKeySource::new(vec![Ok(key.key_set())]);
        let verifier = OidcVerifier::with_source(test_config(), source);

        let wrong_audience = key.sign(
            "https://id.example.test/auth/v1",
            "another-audience",
            "user-6",
        );
        assert!(
            verifier
                .verify(&format!("Bearer {wrong_audience}"))
                .await
                .is_err()
        );

        let wrong_issuer = key.sign("https://evil.example.test", "authenticated", "user-7");
        assert!(
            verifier
                .verify(&format!("Bearer {wrong_issuer}"))
                .await
                .is_err()
        );

        let other_key = TestKey::generate("key-1");
        let forged = other_key.sign("https://id.example.test/auth/v1", "authenticated", "user-8");
        assert!(verifier.verify(&format!("Bearer {forged}")).await.is_err());
    }

    #[tokio::test]
    async fn verify_rejects_unsupported_algorithms_and_missing_kids() {
        let key = TestKey::generate("key-1");
        let source = FakeKeySource::new(vec![Ok(key.key_set())]);
        let verifier = OidcVerifier::with_source(test_config(), source.clone());

        // HS256 is deliberately not accepted even though it carries a kid.
        let hs256 = jsonwebtoken::encode(
            &Header {
                alg: Algorithm::HS256,
                kid: Some("key-1".to_owned()),
                ..Header::default()
            },
            &json!({ "iss": "https://id.example.test/auth/v1", "sub": "user-9" }),
            &EncodingKey::from_secret(b"shared-secret"),
        )
        .expect("hs256 token");
        let error = verifier
            .verify(&format!("Bearer {hs256}"))
            .await
            .expect_err("HS256 must be rejected");
        assert!(error.to_string().contains("algorithm"));
        assert_eq!(source.fetch_count(), 0, "rejected before any key fetch");

        let no_kid = jsonwebtoken::encode(
            &Header {
                alg: Algorithm::ES256,
                ..Header::default()
            },
            &json!({ "iss": "https://id.example.test/auth/v1", "sub": "user-10" }),
            &key.encoding,
        )
        .expect("token without kid");
        let error = verifier
            .verify(&format!("Bearer {no_kid}"))
            .await
            .expect_err("missing kid must be rejected");
        assert!(error.to_string().contains("kid"));
    }

    #[tokio::test]
    async fn verify_rejects_an_empty_key_set() {
        let source = FakeKeySource::new(vec![Ok(JwkSet { keys: Vec::new() })]);
        let verifier = OidcVerifier::with_source(test_config(), source.clone());
        let key = TestKey::generate("key-1");
        let token = key.sign(
            "https://id.example.test/auth/v1",
            "authenticated",
            "user-11",
        );

        let error = verifier
            .verify(&format!("Bearer {token}"))
            .await
            .expect_err("empty key set must fail");

        assert!(error.to_string().contains("no keys"));
        assert_eq!(
            source.fetch_count(),
            1,
            "an empty key set fails before any kid lookup and is never cached"
        );
    }
}
