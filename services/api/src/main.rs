use std::{env, net::SocketAddr};

use anyhow::Context;
use axum::http::{HeaderValue, Uri};
use school_collect_api::{AppState, router};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Environment {
    Development,
    Staging,
    Production,
}

impl Environment {
    fn from_env<F>(mut get: F) -> anyhow::Result<Self>
    where
        F: FnMut(&str) -> Option<String>,
    {
        let value = get("APP_ENV")
            .filter(|value| !value.is_empty())
            .context("APP_ENV is required")?;

        match value.as_str() {
            "development" => Ok(Self::Development),
            "staging" => Ok(Self::Staging),
            "production" => Ok(Self::Production),
            _ => anyhow::bail!("APP_ENV must be development, staging, or production"),
        }
    }

    fn requires_production_configuration(self) -> bool {
        !matches!(self, Self::Development)
    }
}

fn parse_cors_origin(value: &str) -> anyhow::Result<HeaderValue> {
    if value.trim() != value || value.is_empty() {
        anyhow::bail!("APP_CORS_ORIGIN must be an HTTP origin");
    }

    let uri: Uri = value
        .parse()
        .map_err(|_| anyhow::anyhow!("APP_CORS_ORIGIN must be an HTTP origin"))?;
    if !matches!(uri.scheme_str(), Some("http" | "https")) {
        anyhow::bail!("APP_CORS_ORIGIN must use http or https");
    }

    let authority = uri
        .authority()
        .context("APP_CORS_ORIGIN must include a host")?;
    if authority.host().is_empty() || authority.as_str().contains('@') {
        anyhow::bail!("APP_CORS_ORIGIN must be an HTTP origin without credentials");
    }
    let serialized_origin = format!(
        "{}://{}",
        uri.scheme_str().expect("scheme was validated"),
        authority
    );
    if serialized_origin != value {
        anyhow::bail!("APP_CORS_ORIGIN must not include a path or query");
    }

    value
        .parse::<HeaderValue>()
        .map_err(|_| anyhow::anyhow!("APP_CORS_ORIGIN must be an HTTP origin"))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    school_collect_observability::init("api");

    let environment = Environment::from_env(|key| env::var(key).ok())?;
    if environment.requires_production_configuration() {
        school_collect_auth::OidcConfig::from_env(|key| env::var(key).ok())?;
    }

    // Missing configuration is an operator error, not a degraded runtime mode.
    let database_url = env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .context("DATABASE_URL is required to run the API")?;
    let pool = school_collect_db::lazy_pool(&database_url)?;

    let cors_origin_value = env::var("APP_CORS_ORIGIN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            (environment == Environment::Development).then(|| "http://127.0.0.1:1420".to_owned())
        })
        .context("APP_CORS_ORIGIN is required outside development")?;
    let cors_origin = parse_cors_origin(&cors_origin_value)?;

    let address: SocketAddr = env::var("APP_BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_owned())
        .parse()
        .context("APP_BIND_ADDR is not a valid socket address")?;

    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "API listening");

    axum::serve(listener, router(AppState { pool }, cors_origin))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::{Environment, parse_cors_origin};

    #[test]
    fn app_env_is_required_and_rejects_unknown_values_without_echoing_them() {
        assert!(Environment::from_env(|_| None).is_err());
        assert!(Environment::from_env(|_| Some(String::new())).is_err());

        let rejected = "production-with-secret-suffix";
        let error = Environment::from_env(|_| Some(rejected.to_owned()))
            .expect_err("unknown environment must fail")
            .to_string();
        assert!(!error.contains(rejected));
    }

    #[test]
    fn app_env_accepts_only_explicit_known_values() {
        assert_eq!(
            Environment::from_env(|_| Some("development".to_owned())).unwrap(),
            Environment::Development
        );
        assert_eq!(
            Environment::from_env(|_| Some("staging".to_owned())).unwrap(),
            Environment::Staging
        );
        assert_eq!(
            Environment::from_env(|_| Some("production".to_owned())).unwrap(),
            Environment::Production
        );
        assert!(Environment::from_env(|_| Some(" development".to_owned())).is_err());
    }

    #[test]
    fn cors_origin_accepts_serialized_http_origins() {
        for origin in [
            "http://127.0.0.1:1420",
            "https://app.example.test",
            "https://[::1]:1420",
        ] {
            assert_eq!(parse_cors_origin(origin).unwrap(), origin);
        }
    }

    #[test]
    fn cors_origin_rejects_header_junk_and_non_origin_urls() {
        for origin in [
            "junk",
            "null",
            "ftp://app.example.test",
            "https://user@app.example.test",
            "https://app.example.test/",
            "https://app.example.test/path",
            "https://app.example.test?query=true",
            "https://app.example.test#fragment",
            " https://app.example.test",
        ] {
            assert!(
                parse_cors_origin(origin).is_err(),
                "origin should be rejected"
            );
        }
    }

    #[test]
    fn cors_origin_errors_do_not_echo_rejected_values() {
        let rejected = "https://user:secret@app.example.test";
        let error = parse_cors_origin(rejected)
            .expect_err("credential-bearing origin must fail")
            .to_string();
        assert!(!error.contains(rejected));
        assert!(!error.contains("secret"));
    }
}
