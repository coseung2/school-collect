use std::{env, net::SocketAddr};

use anyhow::Context;
use axum::http::HeaderValue;
use school_collect_api::{AppState, router};
use school_collect_auth::{AuthMode, OidcConfig};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Environment {
    Development,
    Staging,
    Production,
}

impl Environment {
    fn from_env() -> anyhow::Result<Self> {
        match env::var("APP_ENV").as_deref().unwrap_or("development") {
            "development" => Ok(Self::Development),
            "staging" => Ok(Self::Staging),
            "production" => Ok(Self::Production),
            value => {
                anyhow::bail!("APP_ENV must be development, staging, or production; got {value}")
            }
        }
    }

    fn requires_production_configuration(self) -> bool {
        !matches!(self, Self::Development)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    school_collect_observability::init("api");

    let environment = Environment::from_env()?;
    let auth_mode = AuthMode::from_env(|key| env::var(key).ok())?;
    let auth_state = match auth_mode {
        AuthMode::DisabledForDevelopment => school_collect_api::AuthState::development_disabled(),
        AuthMode::Oidc => {
            let config = OidcConfig::from_env(|key| env::var(key).ok())?;
            school_collect_api::AuthState::oidc(school_collect_auth::OidcVerifier::new(config)?)
        }
    };
    if environment.requires_production_configuration() && auth_mode != AuthMode::Oidc {
        anyhow::bail!("production-like environments require OIDC authentication");
    }

    // Missing configuration is an operator error, not a degraded runtime mode.
    let database_url = env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .context("DATABASE_URL is required to run the API")?;
    let pool = school_collect_db::lazy_pool(&database_url)?;

    let cors_origin = env::var("APP_CORS_ORIGIN")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            (environment == Environment::Development).then(|| "http://127.0.0.1:1420".to_owned())
        })
        .context("APP_CORS_ORIGIN is required outside development")?
        .parse::<HeaderValue>()
        .context("APP_CORS_ORIGIN is not a valid HTTP origin")?;

    let address: SocketAddr = env::var("APP_BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".to_owned())
        .parse()
        .context("APP_BIND_ADDR is not a valid socket address")?;

    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "API listening");

    axum::serve(listener, router(AppState { pool }, cors_origin, auth_state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
