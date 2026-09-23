use std::sync::Arc;

use anyhow::{Context, bail};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
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

        Ok(Self {
            issuer_url,
            audience,
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

#[derive(Debug, Clone, Deserialize)]
struct JsonWebKeySet {
    keys: Vec<JsonWebKey>,
}

#[derive(Debug, Clone, Deserialize)]
struct JsonWebKey {
    kty: String,
    kid: Option<String>,
    alg: Option<String>,
    n: Option<String>,
    e: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Claims {
    iss: String,
    sub: String,
    #[allow(dead_code)]
    exp: u64,
    #[allow(dead_code)]
    nbf: Option<u64>,
    #[allow(dead_code)]
    aud: serde_json::Value,
}

pub struct OidcVerifier {
    config: OidcConfig,
    client: Client,
    keys: RwLock<Option<JsonWebKeySet>>,
}

impl OidcVerifier {
    pub fn new(config: OidcConfig) -> anyhow::Result<Arc<Self>> {
        Ok(Arc::new(Self {
            config,
            client: Client::builder()
                .https_only(true)
                .build()
                .context("failed to build OIDC HTTP client")?,
            keys: RwLock::new(None),
        }))
    }

    pub async fn verify(&self, authorization: &str) -> anyhow::Result<VerifiedPrincipal> {
        let token = extract_bearer_token(authorization)?;
        let header = decode_header(token).context("invalid JWT header")?;
        if header.alg != Algorithm::RS256 {
            bail!("only RS256 access tokens are accepted");
        }

        let mut keys = self.load_keys(false).await?;
        let key_id = header
            .kid
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("JWT kid is required"))?;
        let key = match keys
            .keys
            .iter()
            .find(|key| key.kid.as_deref() == Some(key_id))
        {
            Some(key) => key,
            None => {
                keys = self.load_keys(true).await?;
                keys.keys
                    .iter()
                    .find(|key| key.kid.as_deref() == Some(key_id))
                    .ok_or_else(|| anyhow::anyhow!("JWT signing key was not found"))?
            }
        };
        if key.kty != "RSA" || key.alg.as_deref() != Some("RS256") {
            bail!("JWT signing key is not an RS256 RSA key");
        }

        let modulus = key
            .n
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("JWT signing key modulus is missing"))?;
        let exponent = key
            .e
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("JWT signing key exponent is missing"))?;
        let decoding_key = DecodingKey::from_rsa_components(modulus, exponent)
            .context("invalid JWT RSA signing key")?;

        let mut validation = Validation::new(Algorithm::RS256);
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
            issuer: claims.iss,
            subject: claims.sub,
        })
    }

    async fn load_keys(&self, force_refresh: bool) -> anyhow::Result<JsonWebKeySet> {
        if !force_refresh && let Some(keys) = self.keys.read().await.clone() {
            return Ok(keys);
        }

        let discovery_url = self
            .config
            .issuer_url
            .join(".well-known/openid-configuration")
            .context("invalid OIDC discovery URL")?;
        let client = &self.client;
        let discovery = client
            .get(discovery_url)
            .send()
            .await
            .context("OIDC discovery request failed")?
            .error_for_status()
            .context("OIDC discovery returned an error")?
            .json::<DiscoveryDocument>()
            .await
            .context("invalid OIDC discovery response")?;

        let jwks_uri = Url::parse(&discovery.jwks_uri).context("invalid OIDC JWKS URL")?;
        if jwks_uri.scheme() != "https" {
            bail!("OIDC JWKS URL must use https");
        }

        let keys = client
            .get(jwks_uri)
            .send()
            .await
            .context("OIDC JWKS request failed")?
            .error_for_status()
            .context("OIDC JWKS returned an error")?
            .json::<JsonWebKeySet>()
            .await
            .context("invalid OIDC JWKS response")?;

        *self.keys.write().await = Some(keys.clone());
        Ok(keys)
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
            ("OIDC_AUDIENCE", "school-collect-api"),
        ]);

        let config = OidcConfig::from_env(|key| values.get(key).map(|value| (*value).to_owned()))
            .expect("valid OIDC configuration");

        assert_eq!(config.audience, "school-collect-api");
        assert_eq!(config.issuer_url.as_str(), "https://id.example.test/");
    }

    #[test]
    fn oidc_config_rejects_non_https_issuer() {
        let values = HashMap::from([
            ("OIDC_ISSUER_URL", "http://id.example.test"),
            ("OIDC_AUDIENCE", "school-collect-api"),
        ]);

        assert!(
            OidcConfig::from_env(|key| values.get(key).map(|value| (*value).to_owned())).is_err()
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
}
