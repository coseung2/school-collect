//! Idempotent handling of delivered outbox events.
//!
//! The database checks run when `DATABASE_URL` is set (the CI `postgres` job and
//! the local Docker stack both provide one) and skip otherwise; the broker check
//! additionally needs `NATS_URL`. Every row the test creates is removed again at
//! the end, including when a check fails.

use std::{
    future::Future,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

use async_nats::jetstream::{self, consumer::PullConsumer};
use school_collect_db::{NewOutboxEvent, insert_outbox, processed_event_count};
use school_collect_worker::{
    ConsumeOutcome, ConsumedEvent, EventEffect, JetStreamConsumer, RelayConfig, consume_batch,
    consume_one,
    nats::{NatsPublisher, STREAM_NAME},
    run_once,
};
use serde_json::json;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

/// An effect that counts its own applications, so a repeated delivery can be
/// shown to repeat nothing.
#[derive(Default)]
struct CountingEffect {
    applied: AtomicUsize,
}

impl CountingEffect {
    fn applied(&self) -> usize {
        self.applied.load(Ordering::SeqCst)
    }
}

impl EventEffect for CountingEffect {
    fn apply<'a>(
        &'a self,
        _connection: &'a mut PgConnection,
        _event: &'a ConsumedEvent,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>> {
        Box::pin(async move {
            self.applied.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
}

/// An effect that fails, so the delivery must fail with it.
struct FailingEffect;

impl EventEffect for FailingEffect {
    fn apply<'a>(
        &'a self,
        _connection: &'a mut PgConnection,
        _event: &'a ConsumedEvent,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>> {
        Box::pin(async { Err(anyhow::anyhow!("the effect failed")) })
    }
}

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

async fn pool() -> Option<PgPool> {
    let url = database_url()?;
    let pool = school_collect_db::connect(&url).await.expect("connect");
    school_collect_db::MIGRATOR
        .run(&pool)
        .await
        .expect("migrate");
    Some(pool)
}

/// A body shaped like the one the relay publishes.
fn body(id: Uuid, topic: &str) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "id": id,
        "type": "collect.published",
        "topic": topic,
        "tenantId": null,
        "aggregateId": null,
        "attempts": 1,
        "payload": { "status": "published", "version": 1 },
    }))
    .expect("body")
}

async fn recorded(pool: &PgPool, consumer: &str, event_id: Uuid) -> Result<i64, String> {
    sqlx::query_scalar(
        "SELECT count(*) FROM school_collect.processed_events
         WHERE consumer = $1 AND event_id = $2",
    )
    .bind(consumer)
    .bind(event_id)
    .fetch_one(pool)
    .await
    .map_err(|error| format!("counting processed events failed: {error}"))
}

async fn remove_processed(pool: &PgPool, consumer: &str) {
    let _ = sqlx::query("DELETE FROM school_collect.processed_events WHERE consumer = $1")
        .bind(consumer)
        .execute(pool)
        .await;
}

#[tokio::test]
async fn a_delivery_is_recorded_once_and_a_repeat_is_a_duplicate() {
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL is not set; skipping the outbox consumer check");
        return;
    };
    let consumer = format!("consumer-test-{}", Uuid::now_v7());
    let event_id = Uuid::now_v7();
    let retried_id = Uuid::now_v7();
    let effect = CountingEffect::default();

    let result: Result<(), String> = async {
        let outcome = consume_one(&pool, &consumer, &effect, &body(event_id, "test.consumer"))
            .await
            .map_err(|error| format!("the first delivery failed: {error}"))?;
        if outcome != ConsumeOutcome::Processed {
            return Err(format!("the first delivery must be processed: {outcome:?}"));
        }
        // The record is committed when the call returns, which is what lets the
        // caller acknowledge the message only afterwards.
        if recorded(&pool, &consumer, event_id).await? != 1 {
            return Err("the processed row must be visible after the call".to_owned());
        }

        let outcome = consume_one(&pool, &consumer, &effect, &body(event_id, "test.consumer"))
            .await
            .map_err(|error| format!("the repeated delivery failed: {error}"))?;
        if outcome != ConsumeOutcome::Duplicate {
            return Err(format!("a repeated delivery is a duplicate: {outcome:?}"));
        }
        if effect.applied() != 1 {
            return Err(format!(
                "the effect must run once, not {} times",
                effect.applied()
            ));
        }
        if processed_event_count(&pool, &consumer)
            .await
            .map_err(|error| format!("counting for the consumer failed: {error}"))?
            != 1
        {
            return Err("a duplicate must not add a row".to_owned());
        }

        // A failing effect rolls its record back with it, so the redelivery the
        // broker schedules can retry the whole delivery instead of skipping it.
        let failed = consume_one(
            &pool,
            &consumer,
            &FailingEffect,
            &body(retried_id, "test.consumer"),
        )
        .await;
        if failed.is_ok() {
            return Err("a failing effect must fail its delivery".to_owned());
        }
        if recorded(&pool, &consumer, retried_id).await? != 0 {
            return Err("a failed delivery must leave no record behind".to_owned());
        }

        let outcome = consume_one(
            &pool,
            &consumer,
            &effect,
            &body(retried_id, "test.consumer"),
        )
        .await
        .map_err(|error| format!("the retry failed: {error}"))?;
        if outcome != ConsumeOutcome::Processed || effect.applied() != 2 {
            return Err("the retried delivery must apply its effect once".to_owned());
        }

        Ok(())
    }
    .await;

    remove_processed(&pool, &consumer).await;

    if let Err(message) = result {
        panic!("outbox consumer verification failed: {message}");
    }
}

/// How many deliveries the durable consumer still holds unacknowledged.
async fn unacknowledged(context: &jetstream::Context, consumer: &str) -> Result<usize, String> {
    let stream = context
        .get_stream(STREAM_NAME)
        .await
        .map_err(|error| format!("the events stream is missing: {error}"))?;
    let handle: PullConsumer = stream
        .get_consumer(consumer)
        .await
        .map_err(|error| format!("the durable consumer is missing: {error}"))?;
    let info = handle
        .get_info()
        .await
        .map_err(|error| format!("the consumer info is missing: {error}"))?;
    Ok(info.num_ack_pending)
}

async fn insert_test_event(pool: &PgPool, name: &str) -> Result<Uuid, String> {
    let mut connection = pool
        .acquire()
        .await
        .map_err(|error| format!("acquiring a connection failed: {error}"))?;
    insert_outbox(
        &mut connection,
        None,
        NewOutboxEvent {
            topic: "test.consumer",
            event_type: name,
            aggregate_id: None,
            payload: &json!({ "name": name }),
        },
    )
    .await
    .map_err(|error| format!("inserting the event failed: {error}"))
}

/// The whole path: the relay publishes, the durable consumer handles the event
/// and acknowledges it only after the record committed.
#[tokio::test]
async fn the_durable_consumer_acknowledges_a_delivery_after_committing_it() {
    let Some(url) = std::env::var("NATS_URL")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
    else {
        eprintln!("NATS_URL is not set; skipping the durable consumer check");
        return;
    };
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL is not set; skipping the durable consumer check");
        return;
    };

    let publisher = NatsPublisher::connect(&url).await.expect("nats");
    let consumer_name = format!("consumer-test-{}", Uuid::now_v7());
    let broker = JetStreamConsumer::from_context(publisher.context().clone(), &consumer_name)
        .await
        .expect("durable consumer");
    let effect = CountingEffect::default();

    let result: Result<(), String> = async {
        let _ = sqlx::query("DELETE FROM school_collect.outbox_events WHERE topic LIKE 'test.%'")
            .execute(&pool)
            .await;
        let event_id = insert_test_event(&pool, "test.consumer").await?;

        let report = run_once(&pool, &publisher, RelayConfig::default())
            .await
            .map_err(|error| format!("the relay round failed: {error}"))?;
        if report.published == 0 {
            return Err("the relay must publish the event".to_owned());
        }

        // Drain whatever the stream holds until this event has been handled.
        for _ in 0..8 {
            let report = consume_batch(&pool, &broker, &effect, 16).await;
            if report.failed > 0 {
                return Err(format!(
                    "a delivery failed ({report:?}): {}",
                    report.last_error.as_deref().unwrap_or("no error recorded")
                ));
            }
            if recorded(&pool, &consumer_name, event_id).await? > 0 {
                break;
            }
        }
        if recorded(&pool, &consumer_name, event_id).await? != 1 {
            return Err("the consumer must handle the published event exactly once".to_owned());
        }
        if processed_event_count(&pool, &consumer_name)
            .await
            .map_err(|error| format!("counting for the consumer failed: {error}"))?
            != 1
        {
            return Err("the consumer must record the event once".to_owned());
        }

        // The acknowledgement is what stops the broker from holding the
        // delivery as pending.
        let mut pending = usize::MAX;
        for _ in 0..50 {
            pending = unacknowledged(publisher.context(), &consumer_name).await?;
            if pending == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        if pending != 0 {
            return Err(format!("the delivery was not acknowledged: {pending}"));
        }

        Ok(())
    }
    .await;

    remove_processed(&pool, &consumer_name).await;
    let _ = sqlx::query("DELETE FROM school_collect.outbox_events WHERE topic LIKE 'test.%'")
        .execute(&pool)
        .await;
    if let Ok(context) = publisher.context().get_stream(STREAM_NAME).await
        && let Err(error) = context.delete_consumer(&consumer_name).await
    {
        eprintln!("the test consumer could not be removed from the broker: {error}");
    }

    if let Err(message) = result {
        panic!("durable consumer verification failed: {message}");
    }
}
