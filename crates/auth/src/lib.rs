use school_collect_domain::{TenantId, UserId};
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
            return Err(anyhow::anyhow!("OIDC_ISSUER_URL must use https"));
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

#[cfg(test)]
mod tests {
    use super::OidcConfig;
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
}
