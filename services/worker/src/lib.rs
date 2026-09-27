//! Outbox relay.
//!
//! Business changes and their events are written in one database transaction;
//! this relay delivers the events afterwards. Delivery is at-least-once: an
//! event is marked published only after the broker acknowledged it, so a crash
//! between publish and acknowledgement repeats the publish instead of losing
//! it. Consumers dedupe by event id, and the JetStream publisher also sets the
//! `Nats-Msg-Id` header so the broker drops duplicates inside its dedup window.

use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use school_collect_db::{
    OutboxEventRecord, OutboxFailureOutcome, claim_outbox_batch, mark_outbox_failed,
    mark_outbox_published,
};
use sqlx::PgPool;

pub mod nats;

pub mod attachments;

pub mod consumer;

pub use attachments::{PurgeReport, purge_expired_attachments, run_sweep_until_shutdown};

pub use consumer::{
    CONSUMER_NAME, ConsumeOutcome, ConsumedEvent, ConsumerReport, EventEffect, JetStreamConsumer,
    RecordOnlyEffect, consume_batch, consume_one, run_consumer_until_shutdown,
};

/// How the relay delivers one event.
pub trait EventPublisher: Send + Sync {
    fn publish<'a>(
        &'a self,
        event: &'a OutboxEventRecord,
    ) -> Pin<Box<dyn Future<Output = Result<(), PublishFailure>> + Send + 'a>>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishFailure {
    pub message: String,
}

impl PublishFailure {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayConfig {
    /// Events claimed per round.
    pub batch_size: i64,
    /// Attempts allowed before an event is dead-lettered.
    pub max_attempts: i32,
    /// Base delay between attempts; grows with the attempt count.
    pub retry_delay: Duration,
    /// How long a claimed batch is reserved for this relay.
    pub claim_lease: Duration,
}

impl Default for RelayConfig {
    fn default() -> Self {
        Self {
            batch_size: 20,
            max_attempts: 5,
            retry_delay: Duration::from_secs(5),
            claim_lease: Duration::from_secs(60),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RelayReport {
    pub claimed: usize,
    pub published: usize,
    pub retried: usize,
    pub dead_lettered: usize,
}

/// Claims one batch and delivers it.
///
/// The batch is claimed before publishing so two relays never take the same
/// event, and each result is written back before the next round.
pub async fn run_once(
    pool: &PgPool,
    publisher: &dyn EventPublisher,
    config: RelayConfig,
) -> anyhow::Result<RelayReport> {
    let events = claim_outbox_batch(pool, config.batch_size, config.claim_lease).await?;
    let mut report = RelayReport {
        claimed: events.len(),
        ..RelayReport::default()
    };

    for event in events {
        match publisher.publish(&event).await {
            Ok(()) => {
                mark_outbox_published(pool, event.id).await?;
                report.published += 1;
                tracing::info!(
                    event.id = %event.id,
                    event.type = %event.event_type,
                    event.topic = %event.topic,
                    "outbox event delivered"
                );
            }
            Err(failure) => {
                let outcome = mark_outbox_failed(
                    pool,
                    event.id,
                    &failure.message,
                    config.max_attempts,
                    config.retry_delay.as_millis() as i64,
                )
                .await?;
                match outcome {
                    OutboxFailureOutcome::Retried { attempts } => {
                        report.retried += 1;
                        tracing::warn!(
                            event.id = %event.id,
                            attempts,
                            reason = %failure.message,
                            "outbox delivery failed; retry scheduled"
                        );
                    }
                    OutboxFailureOutcome::DeadLettered { attempts } => {
                        report.dead_lettered += 1;
                        tracing::error!(
                            event.id = %event.id,
                            attempts,
                            reason = %failure.message,
                            "outbox delivery failed permanently"
                        );
                    }
                }
            }
        }
    }

    Ok(report)
}

/// Runs the relay until the shutdown future resolves.
pub async fn run_until_shutdown(
    pool: PgPool,
    publisher: Arc<dyn EventPublisher>,
    config: RelayConfig,
    poll_interval: Duration,
    shutdown: impl Future<Output = ()>,
) {
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            () = &mut shutdown => {
                tracing::info!("outbox relay stopping");
                return;
            }
            result = run_once(&pool, publisher.as_ref(), config) => {
                match result {
                    Ok(report) if report.claimed == 0 => {}
                    Ok(report) => {
                        tracing::info!(
                            claimed = report.claimed,
                            published = report.published,
                            retried = report.retried,
                            dead_lettered = report.dead_lettered,
                            "outbox batch processed"
                        );
                    }
                    Err(error) => {
                        tracing::error!(%error, "outbox relay round failed");
                    }
                }
            }
            () = tokio::time::sleep(poll_interval) => {}
        }
    }
}
