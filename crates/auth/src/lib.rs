use school_collect_domain::{TenantId, UserId};
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

        Ok(Self { issuer, audience })
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
        assert_eq!(config.issuer, "https://id.example.test");
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
    fn oidc_config_preserves_path_and_trailing_slash_exactly() {
        for issuer in [
            "https://id.example.test",
            "https://id.example.test/",
            "https://id.example.test/oauth",
        ] {
            let values = HashMap::from([
                ("OIDC_ISSUER_URL", issuer),
                ("OIDC_AUDIENCE", "school-collect-api"),
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
        ]);

        let error = OidcConfig::from_env(|key| values.get(key).map(|value| (*value).to_owned()))
            .expect_err("credentials must be rejected")
            .to_string();

        assert!(!error.contains(sensitive_issuer));
        assert!(!error.contains("secret"));
    }
}
