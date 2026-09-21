use std::{env, net::SocketAddr};

use anyhow::Context;
use axum::http::HeaderValue;
use school_collect_api::{router, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    school_collect_observability::init("api");

    let pool = match env::var("DATABASE_URL") {
        Ok(database_url) => Some(school_collect_db::connect(&database_url).await?),
        Err(_) => {
            tracing::warn!("DATABASE_URL is unset; API will start but readiness will fail");
            None
        }
    };

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
