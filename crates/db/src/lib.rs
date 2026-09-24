use std::time::Duration;

use anyhow::Context;
use sqlx::{PgConnection, PgPool, postgres::PgPoolOptions};

pub mod records;

pub use records::*;

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../../migrations/v2");

/// Dedicated schema for v2 objects.
///
/// The retired v1 application left tables behind in `public`, including a
/// `collect_submissions` table with a different shape. Keeping v2 objects in
/// their own schema makes an accidental read of legacy data impossible.
pub const SCHEMA: &str = "school_collect";

/// Every pooled connection resolves unqualified names to the v2 schema first.
const SEARCH_PATH: &str = "school_collect,public";

async fn configure_connection(connection: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query(&format!("SET search_path TO {SEARCH_PATH}"))
        .execute(connection)
        .await?;
    Ok(())
}

pub async fn connect(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .after_connect(|connection, _meta| {
            Box::pin(async move {
                configure_connection(connection).await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await
        .context("failed to connect to PostgreSQL")?;
    Ok(pool)
}

/// Builds a pool without requiring PostgreSQL to be reachable yet.
///
/// Startup must not silently continue with missing configuration, but a
/// database that is merely unavailable belongs to readiness, not startup.
pub fn lazy_pool(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(2))
        .after_connect(|connection, _meta| {
            Box::pin(async move {
                configure_connection(connection).await?;
                Ok(())
            })
        })
        .connect_lazy(database_url)
        .context("DATABASE_URL is not a valid PostgreSQL connection string")?;
    Ok(pool)
}

pub async fn ping(pool: &PgPool) -> anyhow::Result<()> {
    sqlx::query("SELECT 1")
        .execute(pool)
        .await
        .context("PostgreSQL readiness query failed")?;
    Ok(())
}

/// Readiness for the v2 schema, not just for an open socket.
///
/// A reachable database that has not been migrated yet must not report ready,
/// otherwise the client would receive "successful" responses for routes whose
/// tables do not exist.
pub async fn verify_schema(pool: &PgPool) -> anyhow::Result<()> {
    let present: Option<String> = sqlx::query_scalar("SELECT to_regclass($1)::text")
        .bind(format!("{SCHEMA}.collects"))
        .fetch_one(pool)
        .await
        .context("PostgreSQL schema verification query failed")?;

    match present {
        Some(_) => Ok(()),
        None => anyhow::bail!("the {SCHEMA} schema has not been migrated yet"),
    }
}
