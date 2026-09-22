use std::time::Duration;

use anyhow::Context;
use sqlx::{PgPool, postgres::PgPoolOptions};

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations/v2");

pub async fn connect(database_url: &str) -> anyhow::Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
        .context("failed to connect to PostgreSQL")
}

/// Builds a pool without requiring PostgreSQL to be reachable yet.
///
/// Startup must not silently continue with missing configuration, but a
/// database that is merely unavailable belongs to readiness, not startup.
pub fn lazy_pool(database_url: &str) -> anyhow::Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(2))
        .connect_lazy(database_url)
        .context("DATABASE_URL is not a valid PostgreSQL connection string")
}

pub async fn ping(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .context("PostgreSQL readiness query failed")?;
    Ok(())
}
