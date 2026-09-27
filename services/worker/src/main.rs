//! Outbox relay process.
//!
//! Both dependencies are required: missing configuration must fail startup
//! rather than silently drop events.

use std::{env, sync::Arc, time::Duration};

use anyhow::Context;
use school_collect_application::storage::{ObjectStorage, configured_storage};
use school_collect_worker::{
    CONSUMER_NAME, EventEffect, JetStreamConsumer, RecordOnlyEffect, RelayConfig,
    nats::NatsPublisher, run_consumer_until_shutdown, run_sweep_until_shutdown, run_until_shutdown,
};

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
    // Schema changes belong to `school-collect-migrator` alone, so the worker
    // can run with a role that has no DDL rights. It only checks that the
    // migrator already ran.
    school_collect_db::verify_schema(&pool)
        .await
        .context("the database schema is not ready; run school-collect-migrator first")?;
    let publisher = Arc::new(NatsPublisher::connect(&nats_url).await?);
    tracing::info!(
        stream = school_collect_worker::nats::STREAM_NAME,
        "outbox relay started"
    );

    // The consumer handles the events the relay has published. It shares the
    // relay's connection and writes its progress under the durable name, so one
    // worker process both delivers the outbox and consumes it.
    let consumer = Arc::new(
        JetStreamConsumer::from_context(publisher.context().clone(), CONSUMER_NAME).await?,
    );
    let consumer_batch = read_number("WORKER_CONSUMER_BATCH", 50)?;
    let consumer_idle = Duration::from_millis(read_number("WORKER_CONSUMER_IDLE_MS", 1_000)?);
    let consumer_backoff = Duration::from_millis(read_number("WORKER_CONSUMER_BACKOFF_MS", 5_000)?);
    let effect: Arc<dyn EventEffect> = Arc::new(RecordOnlyEffect);
    tracing::info!(consumer = CONSUMER_NAME, "outbox consumer started");

    // The retention sweep needs to reach the same object storage the API writes
    // to. Without that configuration the relay still runs; attachments simply
    // keep their rows until an operator points the worker at the store.
    let storage: Option<Arc<dyn ObjectStorage>> = match configured_storage()
        .map_err(|error| anyhow::anyhow!("attachment storage: {error}"))?
    {
        Some((storage, kind)) => {
            tracing::info!(storage = kind, "attachment retention sweep enabled");
            Some(storage)
        }
        None => {
            tracing::info!("no attachment storage is set; the attachment retention sweep is off");
            None
        }
    };
    let sweep_interval = Duration::from_millis(read_number(
        "WORKER_ATTACHMENT_SWEEP_MS",
        6 * 60 * 60 * 1_000,
    )?);
    let sweep_batch = read_number("WORKER_ATTACHMENT_BATCH", 100)?;

    let sweep = storage.map(|storage| {
        let pool = pool.clone();
        tokio::spawn(run_sweep_until_shutdown(
            pool,
            storage,
            sweep_batch,
            sweep_interval,
            async {
                let _ = tokio::signal::ctrl_c().await;
            },
        ))
    });

    let consuming = tokio::spawn(run_consumer_until_shutdown(
        pool.clone(),
        consumer,
        effect,
        consumer_batch,
        consumer_idle,
        consumer_backoff,
        async {
            let _ = tokio::signal::ctrl_c().await;
        },
    ));

    run_until_shutdown(pool, publisher, config, poll_interval, async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await;

    if let Some(sweep) = sweep {
        let _ = sweep.await;
    }
    let _ = consuming.await;

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
