//! Idempotency keys for delivered events.
//!
//! Delivery is at-least-once, so a consumer needs a durable record of what it
//! already handled. The row is written inside the same transaction as the
//! effect it describes, which is what makes "handled once" and "effect applied
//! once" the same commit. The table is operational bookkeeping: it carries no
//! tenant and no payload.

use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

/// Records that `consumer` handled `event_id`.
///
/// Returns whether this delivery was the first one. `false` means the event was
/// handled before, so the caller can acknowledge the repeated delivery without
/// repeating its effect.
pub async fn record_event_processed(
    connection: &mut PgConnection,
    consumer: &str,
    event_id: Uuid,
    topic: &str,
) -> anyhow::Result<bool> {
    let inserted = sqlx::query(
        "INSERT INTO school_collect.processed_events (id, consumer, event_id, topic)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (consumer, event_id) DO NOTHING",
    )
    .bind(Uuid::now_v7())
    .bind(consumer)
    .bind(event_id)
    .bind(topic)
    .execute(connection)
    .await?
    .rows_affected();
    Ok(inserted == 1)
}

/// How many deliveries one consumer has handled.
///
/// The unique key `(consumer, event_id)` serves this lookup, so operators can
/// read a consumer's progress without another index.
pub async fn processed_event_count(pool: &PgPool, consumer: &str) -> anyhow::Result<i64> {
    let count = sqlx::query_scalar(
        "SELECT count(*) FROM school_collect.processed_events WHERE consumer = $1",
    )
    .bind(consumer)
    .fetch_one(pool)
    .await?;
    Ok(count)
}
