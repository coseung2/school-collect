use std::{env, net::SocketAddr};

use anyhow::Context;
use axum::http::HeaderValue;
use school_collect_api::{AppState, router};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    school_collect_observability::init("api");

    // Missing configuration is an operator error, not a degraded runtime mode.
    let database_url = env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .context("DATABASE_URL is required to run the API")?;
    let pool = school_collect_db::lazy_pool(&database_url)?;

    let cors_origin = env::var("APP_CORS_ORIGIN")
        .unwrap_or_else(|_| "http://127.0.0.1:1420".to_owned())
        .parse::<HeaderValue>()
        .context("APP_CORS_ORIGIN is not a valid HTTP origin")?;

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
