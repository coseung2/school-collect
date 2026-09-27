//! Outbox relay process.
//!
//! Both dependencies are required: missing configuration must fail startup
//! rather than silently drop events.

use std::{env, sync::Arc, time::Duration};

use anyhow::Context;
use school_collect_worker::{RelayConfig, nats::NatsPublisher, run_until_shutdown};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    school_collect_observability::init("worker");

    let database_url = env::var("DATABASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .context("DATABASE_URL is required to run the worker")?;
    let nats_url = env::var("NATS_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .context("NATS_URL is required to run the worker")?;

    let config = RelayConfig {
        batch_size: read_number("WORKER_BATCH_SIZE", 20)?,
        max_attempts: read_number("WORKER_MAX_ATTEMPTS", 5)?,
        retry_delay: Duration::from_millis(read_number("WORKER_RETRY_DELAY_MS", 5_000)?),
        claim_lease: Duration::from_millis(read_number("WORKER_CLAIM_LEASE_MS", 60_000)?),
    };
    let poll_interval = Duration::from_millis(read_number("WORKER_POLL_MS", 1_000)?);

    let pool = school_collect_db::connect(&database_url).await?;
    school_collect_db::MIGRATOR
        .run(&pool)
        .await
        .context("failed to apply migrations")?;
    let publisher = Arc::new(NatsPublisher::connect(&nats_url).await?);
    tracing::info!(
        stream = school_collect_worker::nats::STREAM_NAME,
        "outbox relay started"
    );

    run_until_shutdown(pool, publisher, config, poll_interval, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await;

    Ok(())
}

fn read_number<T>(name: &str, default: T) -> anyhow::Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => value
            .trim()
            .parse::<T>()
            .map_err(|error| anyhow::anyhow!("{name} is invalid: {error}")),
        _ => Ok(default),
    }
}
