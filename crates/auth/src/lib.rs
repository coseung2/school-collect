use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use jsonwebtoken::{
    Algorithm, DecodingKey, Validation, decode, decode_header,
    jwk::{Jwk, JwkSet, KeyAlgorithm, KeyOperations, PublicKeyUse},
};
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
    /// Exact issuer identifier configured by the operator.
    ///
    /// This remains a string because OIDC issuer comparison is exact. URL
    /// serialization can otherwise add a trailing slash or normalize casing.
    pub issuer: String,
    pub audience: String,
    pub jwks_url: Url,
}

impl OidcConfig {
    pub fn from_env<F>(mut get: F) -> anyhow::Result<Self>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let issuer = get("OIDC_ISSUER_URL")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("OIDC_ISSUER_URL is required"))?;
        if issuer.trim() != issuer {
            return Err(anyhow::anyhow!(
                "OIDC_ISSUER_URL must not contain surrounding whitespace"
            ));
        }

        let issuer_url = Url::parse(&issuer)
            .map_err(|_| anyhow::anyhow!("OIDC_ISSUER_URL is not a valid URL"))?;
        if issuer_url.scheme() != "https" {
            return Err(anyhow::anyhow!("OIDC_ISSUER_URL must use https"));
        }
        if issuer_url.host().is_none() {
            return Err(anyhow::anyhow!("OIDC_ISSUER_URL must include a host"));
        }
        if !issuer_url.username().is_empty() || issuer_url.password().is_some() {
            return Err(anyhow::anyhow!(
                "OIDC_ISSUER_URL must not include credentials"
            ));
        }
        if issuer_url.query().is_some() {
            return Err(anyhow::anyhow!("OIDC_ISSUER_URL must not include a query"));
        }
        if issuer_url.fragment().is_some() {
            return Err(anyhow::anyhow!(
                "OIDC_ISSUER_URL must not include a fragment"
            ));
        }

        let audience = get("OIDC_AUDIENCE")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("OIDC_AUDIENCE is required"))?;
        if audience.trim() != audience {
            return Err(anyhow::anyhow!(
                "OIDC_AUDIENCE must not contain surrounding whitespace"
            ));
        }

        let jwks_url = get("OIDC_JWKS_URL")
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("OIDC_JWKS_URL is required"))?;
        let jwks_url = parse_https_endpoint(&jwks_url, "OIDC_JWKS_URL")?;

        Ok(Self {
            issuer,
            audience,
            jwks_url,
        })
    }
}

fn parse_https_endpoint(value: &str, name: &str) -> anyhow::Result<Url> {
    if value.trim() != value {
        return Err(anyhow::anyhow!(
            "{name} must not contain surrounding whitespace"
        ));
    }

    let url = Url::parse(value).map_err(|_| anyhow::anyhow!("{name} is not a valid URL"))?;
    if url.scheme() != "https" {
        return Err(anyhow::anyhow!("{name} must use https"));
    }
    if url.host().is_none() {
        return Err(anyhow::anyhow!("{name} must include a host"));
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err(anyhow::anyhow!("{name} must not include credentials"));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(anyhow::anyhow!(
            "{name} must not include a query or fragment"
        ));
    }

    Ok(url)
}

#[derive(Debug, Clone, Deserialize)]
pub struct AccessTokenClaims {
    pub sub: String,
    pub iss: String,
    pub exp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    pub subject: String,
    pub expires_at: u64,
}

struct CachedJwks {
    keys: JwkSet,
    fetched_at: Instant,
}

/// Verifies OIDC access tokens against the configured issuer and a cached JWKS.
///
/// The key set is refreshed on a five-minute cadence and immediately retried
/// once when a token references an unknown key id, which supports normal key
/// rotation without trusting a token until its signature is verified.
#[derive(Clone)]
pub struct TokenVerifier {
    config: OidcConfig,
    client: Client,
    cache: Arc<RwLock<Option<CachedJwks>>>,
}

impl TokenVerifier {
    const JWKS_CACHE_TTL: Duration = Duration::from_secs(300);

    pub fn new(config: OidcConfig) -> anyhow::Result<Self> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(5))
            .build()
            .context("failed to create OIDC JWKS client")?;

        Ok(Self {
            config,
            client,
            cache: Arc::new(RwLock::new(None)),
        })
    }

    pub async fn verify_bearer(&self, authorization: &str) -> anyhow::Result<Principal> {
        let mut parts = authorization.split_ascii_whitespace();
        let scheme = parts.next();
        let token = parts.next();
        if !matches!(scheme, Some(value) if value.eq_ignore_ascii_case("bearer"))
            || token.is_none()
            || parts.next().is_some()
        {
            anyhow::bail!("authorization header is invalid")
        }

        self.verify_token(token.expect("token was checked above"))
            .await
    }

    async fn verify_token(&self, token: &str) -> anyhow::Result<Principal> {
        let header =
            decode_header(token).map_err(|_| anyhow::anyhow!("access token is invalid"))?;
        if header.alg != Algorithm::RS256 {
            anyhow::bail!("access token algorithm is not allowed")
        }
        let kid = header
            .kid
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("access token key id is missing"))?;

        let jwk = self.find_key(&kid).await?;
        if jwk.common.key_algorithm != Some(KeyAlgorithm::RS256)
            || matches!(jwk.common.public_key_use, Some(PublicKeyUse::Encryption))
            || matches!(
                jwk.common.key_operations.as_ref(),
                Some(operations) if !operations.iter().any(|operation| matches!(operation, KeyOperations::Verify))
            )
        {
            anyhow::bail!("OIDC signing key is not valid for token verification")
        }

        let key = DecodingKey::from_jwk(&jwk)
            .map_err(|_| anyhow::anyhow!("OIDC signing key is invalid"))?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.set_issuer(&[self.config.issuer.as_str()]);
        validation.set_audience(&[self.config.audience.as_str()]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);

        let claims = decode::<AccessTokenClaims>(token, &key, &validation)
            .map_err(|_| anyhow::anyhow!("access token is invalid"))?
            .claims;
        if claims.sub.trim().is_empty() {
            anyhow::bail!("access token subject is missing")
        }

        Ok(Principal {
            subject: claims.sub,
            expires_at: claims.exp,
        })
    }

    async fn find_key(&self, kid: &str) -> anyhow::Result<Jwk> {
        if let Some(key) = self.cached_key(kid).await {
            return Ok(key);
        }

        let keys = self.fetch_jwks().await?;
        let key = keys
            .find(kid)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("OIDC signing key is not available"))?;
        *self.cache.write().await = Some(CachedJwks {
            keys,
            fetched_at: Instant::now(),
        });
        Ok(key)
    }

    async fn cached_key(&self, kid: &str) -> Option<Jwk> {
        let cache = self.cache.read().await;
        let cached = cache.as_ref()?;
        if cached.fetched_at.elapsed() >= Self::JWKS_CACHE_TTL {
            return None;
        }
        cached.keys.find(kid).cloned()
    }

    async fn fetch_jwks(&self) -> anyhow::Result<JwkSet> {
        let response = self
            .client
            .get(self.config.jwks_url.clone())
            .send()
            .await
            .map_err(|_| anyhow::anyhow!("OIDC JWKS request failed"))?;
        if !response.status().is_success() {
            anyhow::bail!("OIDC JWKS request failed")
        }

        let keys = response
            .json::<JwkSet>()
            .await
            .map_err(|_| anyhow::anyhow!("OIDC JWKS response is invalid"))?;
        if keys.keys.is_empty() {
            anyhow::bail!("OIDC JWKS response has no signing keys")
        }
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::{OidcConfig, TokenVerifier};
    use std::collections::HashMap;

    #[test]
    fn oidc_config_requires_https_issuer_and_audience() {
        let values = HashMap::from([
            ("OIDC_ISSUER_URL", "https://id.example.test"),
            ("OIDC_AUDIENCE", "school-collect-api"),
            ("OIDC_JWKS_URL", "https://id.example.test/oauth/keys"),
        ]);

        let config = OidcConfig::from_env(|key| values.get(key).map(|value| (*value).to_owned()))
            .expect("valid OIDC configuration");

        assert_eq!(config.audience, "school-collect-api");
        assert_eq!(config.issuer, "https://id.example.test");
        assert_eq!(
            config.jwks_url.as_str(),
            "https://id.example.test/oauth/keys"
        );
    }

    #[test]
    fn oidc_config_rejects_non_https_issuer() {
        let values = HashMap::from([
            ("OIDC_ISSUER_URL", "http://id.example.test"),
            ("OIDC_AUDIENCE", "school-collect-api"),
            ("OIDC_JWKS_URL", "https://id.example.test/oauth/keys"),
        ]);

        assert!(
            OidcConfig::from_env(|key| values.get(key).map(|value| (*value).to_owned())).is_err()
        );
    }

    #[test]
    fn oidc_config_preserves_path_and_trailing_slash_exactly() {
        for issuer in [
            "https://id.example.test",
            "https://id.example.test/",
            "https://id.example.test/oauth",
        ] {
            let values = HashMap::from([
                ("OIDC_ISSUER_URL", issuer),
                ("OIDC_AUDIENCE", "school-collect-api"),
                ("OIDC_JWKS_URL", "https://id.example.test/oauth/keys"),
            ]);

            let config =
                OidcConfig::from_env(|key| values.get(key).map(|value| (*value).to_owned()))
                    .expect("valid OIDC configuration");

            assert_eq!(config.issuer, issuer);
        }
    }

    #[test]
    fn oidc_config_rejects_issuer_credentials_query_fragment_and_whitespace() {
        for issuer in [
            "https://user@id.example.test",
            "https://user:password@id.example.test",
            "https://id.example.test?tenant=example",
            "https://id.example.test#issuer",
            " https://id.example.test",
            "https://id.example.test ",
        ] {
            let values = HashMap::from([
                ("OIDC_ISSUER_URL", issuer),
                ("OIDC_AUDIENCE", "school-collect-api"),
                ("OIDC_JWKS_URL", "https://id.example.test/oauth/keys"),
            ]);

            assert!(
                OidcConfig::from_env(|key| values.get(key).map(|value| (*value).to_owned()))
                    .is_err(),
                "issuer should be rejected"
            );
        }
    }

    #[test]
    fn oidc_config_errors_do_not_include_rejected_values() {
        let sensitive_issuer = "https://user:secret@id.example.test";
        let values = HashMap::from([
            ("OIDC_ISSUER_URL", sensitive_issuer),
            ("OIDC_AUDIENCE", "school-collect-api"),
            ("OIDC_JWKS_URL", "https://id.example.test/oauth/keys"),
        ]);

        let error = OidcConfig::from_env(|key| values.get(key).map(|value| (*value).to_owned()))
            .expect_err("credentials must be rejected")
            .to_string();

        assert!(!error.contains(sensitive_issuer));
        assert!(!error.contains("secret"));
    }

    #[test]
    fn oidc_config_rejects_invalid_jwks_endpoint() {
        for endpoint in [
            "http://id.example.test/oauth/keys",
            "https://user:secret@id.example.test/oauth/keys",
            "https://id.example.test/oauth/keys?tenant=one",
            " https://id.example.test/oauth/keys",
        ] {
            let values = HashMap::from([
                ("OIDC_ISSUER_URL", "https://id.example.test"),
                ("OIDC_AUDIENCE", "school-collect-api"),
                ("OIDC_JWKS_URL", endpoint),
            ]);

            assert!(
                OidcConfig::from_env(|key| values.get(key).map(|value| (*value).to_owned()))
                    .is_err(),
                "JWKS endpoint should be rejected"
            );
        }
    }

    #[tokio::test]
    async fn bearer_parser_rejects_malformed_authorization_without_network_access()
    -> anyhow::Result<()> {
        let values = HashMap::from([
            ("OIDC_ISSUER_URL", "https://id.example.test"),
            ("OIDC_AUDIENCE", "school-collect-api"),
            ("OIDC_JWKS_URL", "https://id.example.test/oauth/keys"),
        ]);
        let verifier = TokenVerifier::new(OidcConfig::from_env(|key| {
            values.get(key).map(|value| (*value).to_owned())
        })?)?;

        for header in ["", "Basic token", "Bearer", "Bearer one two"] {
            assert!(verifier.verify_bearer(header).await.is_err());
        }
        Ok(())
    }
}
