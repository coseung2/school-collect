use std::sync::Arc;

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
    client: Client,
    keys: RwLock<Option<JwkSet>>,
}

impl OidcVerifier {
    pub fn new(config: OidcConfig) -> anyhow::Result<Arc<Self>> {
        Ok(Arc::new(Self {
            config,
            client: Client::builder()
                .https_only(true)
                .user_agent(concat!("school-collect-api/", env!("CARGO_PKG_VERSION")))
                .build()
                .context("failed to build OIDC HTTP client")?,
            keys: RwLock::new(None),
        }))
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

        let jwks_url = match &self.config.jwks_url {
            Some(url) => url.clone(),
            None => {
                // `Url::join` would replace the final path segment of an issuer
                // such as `https://host/auth/v1`, so the discovery path is
                // appended explicitly instead.
                let discovery_url = self.discovery_url().context("invalid OIDC discovery URL")?;
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

        if keys.keys.is_empty() {
            bail!("OIDC JWKS contained no keys");
        }

        *self.keys.write().await = Some(keys.clone());
        Ok(keys)
    }

    fn discovery_url(&self) -> anyhow::Result<Url> {
        let base = self.config.issuer_url.as_str().trim_end_matches('/');
        Url::parse(&format!("{base}/.well-known/openid-configuration"))
            .context("OIDC discovery URL is not a valid URL")
    }
}

#[cfg(test)]
mod tests {
    use super::{AuthMode, OidcConfig, extract_bearer_token};
    use std::collections::HashMap;

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
        let verifier = super::OidcVerifier::new(config).unwrap();

        assert_eq!(
            verifier.discovery_url().unwrap().as_str(),
            "https://id.example.test/auth/v1/.well-known/openid-configuration"
        );
    }
}
