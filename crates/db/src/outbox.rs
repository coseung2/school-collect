//! Transactional outbox: write events with the business change, deliver them
//! afterwards.
//!
//! The relay claims a batch with `FOR UPDATE SKIP LOCKED`, so several workers
//! can run at once without handing the same event to two of them. A row is
//! marked published only after the broker acknowledged the publish; failures
//! schedule a retry and, once the attempts run out, dead-letter the row.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgConnection, PgPool, Row, postgres::PgRow};
use uuid::Uuid;

/// Event written next to a business change.
pub struct NewOutboxEvent<'a> {
    /// Stream the event belongs to, for example `school_collect.collects`.
    pub topic: &'a str,
    /// Business event name, for example `collect.published`.
    pub event_type: &'a str,
    /// Row the event is about.
    pub aggregate_id: Option<Uuid>,
    /// Event body. Must not contain personal data beyond identifiers.
    pub payload: &'a Value,
}

/// Writes an event inside the caller's transaction.
///
/// Taking the open connection (rather than the pool) is what makes the outbox
/// transactional: if the business change rolls back, the event disappears with
/// it.
pub async fn insert_outbox(
    connection: &mut PgConnection,
    tenant_id: Option<Uuid>,
    event: NewOutboxEvent<'_>,
) -> anyhow::Result<Uuid> {
    let id = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO school_collect.outbox_events
           (id, tenant_id, topic, event_type, aggregate_id, payload)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(id)
    .bind(tenant_id)
    .bind(event.topic)
    .bind(event.event_type)
    .bind(event.aggregate_id)
    .bind(event.payload)
    .execute(connection)
    .await?;
    Ok(id)
}

/// One claimed event, ready to publish.
#[derive(Debug, Clone)]
pub struct OutboxEventRecord {
    pub id: Uuid,
    pub tenant_id: Option<Uuid>,
    pub topic: String,
    pub event_type: String,
    pub aggregate_id: Option<Uuid>,
    pub payload: Value,
    pub attempts: i32,
}

impl OutboxEventRecord {
    fn from_row(row: &PgRow) -> Self {
        Self {
            id: row.get("id"),
            tenant_id: row.get("tenant_id"),
            topic: row.get("topic"),
            event_type: row.get("event_type"),
            aggregate_id: row.get("aggregate_id"),
            payload: row.get("payload"),
            attempts: row.get("attempts"),
        }
    }
}

/// Claims up to `limit` due events and counts the attempt.
///
/// `SKIP LOCKED` keeps concurrent relays from blocking each other, and the
/// attempt counter moves before publishing so a worker that dies mid-publish
/// does not retry forever.
pub async fn claim_outbox_batch(
    pool: &PgPool,
    limit: i64,
) -> anyhow::Result<Vec<OutboxEventRecord>> {
    let rows = sqlx::query(
        "UPDATE school_collect.outbox_events AS claimed
         SET attempts = claimed.attempts + 1
         WHERE claimed.id IN (
             SELECT pending.id
             FROM school_collect.outbox_events pending
             WHERE pending.published_at IS NULL
               AND pending.dead_lettered_at IS NULL
               AND pending.available_at <= now()
             ORDER BY pending.available_at, pending.created_at
             LIMIT $1
             FOR UPDATE SKIP LOCKED
         )
         RETURNING claimed.id, claimed.tenant_id, claimed.topic, claimed.event_type,
                   claimed.aggregate_id, claimed.payload, claimed.attempts",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(rows.iter().map(OutboxEventRecord::from_row).collect())
}

/// Marks an event delivered. The row stays for the audit trail.
pub async fn mark_outbox_published(pool: &PgPool, id: Uuid) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE school_collect.outbox_events
         SET published_at = now(), last_error = NULL
         WHERE id = $1",
    )
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

/// What happened to a failed delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxFailureOutcome {
    /// Scheduled for another attempt.
    Retried { attempts: i32 },
    /// Attempts are exhausted; the row is parked for operators.
    DeadLettered { attempts: i32 },
}

/// Records a failed delivery and schedules the next attempt.
///
/// The delay grows with the attempt count so a broken dependency is not hit in
/// a tight loop, and the row is dead-lettered once `max_attempts` is reached.
pub async fn mark_outbox_failed(
    pool: &PgPool,
    id: Uuid,
    error: &str,
    max_attempts: i32,
    retry_delay_ms: i64,
) -> anyhow::Result<OutboxFailureOutcome> {
    let attempts = sqlx::query_scalar::<_, i32>(
        "SELECT attempts FROM school_collect.outbox_events WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .unwrap_or(max_attempts);

    let error = truncate_error(error);
    if attempts >= max_attempts {
        sqlx::query(
            "UPDATE school_collect.outbox_events
             SET dead_lettered_at = now(), last_error = $2
             WHERE id = $1",
        )
        .bind(id)
        .bind(error)
        .execute(pool)
        .await?;
        return Ok(OutboxFailureOutcome::DeadLettered { attempts });
    }

    let delay = retry_delay_ms.saturating_mul(i64::from(attempts.max(1)));
    sqlx::query(
        "UPDATE school_collect.outbox_events
         SET available_at = now() + make_interval(secs => $2), last_error = $3
         WHERE id = $1",
    )
    .bind(id)
    .bind(delay as f64 / 1000.0)
    .bind(error)
    .execute(pool)
    .await?;
    Ok(OutboxFailureOutcome::Retried { attempts })
}

fn truncate_error(error: &str) -> String {
    const MAX: usize = 500;
    error.chars().take(MAX).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OutboxStats {
    pub pending: i64,
    pub published: i64,
    pub dead_lettered: i64,
}

/// Delivery counters for the readiness screen and operators.
pub async fn outbox_stats(pool: &PgPool) -> anyhow::Result<OutboxStats> {
    let row = sqlx::query(
        "SELECT
           count(*) FILTER (WHERE published_at IS NULL AND dead_lettered_at IS NULL) AS pending,
           count(*) FILTER (WHERE published_at IS NOT NULL) AS published,
           count(*) FILTER (WHERE dead_lettered_at IS NOT NULL) AS dead_lettered
         FROM school_collect.outbox_events",
    )
    .fetch_one(pool)
    .await?;

    Ok(OutboxStats {
        pending: row.get("pending"),
        published: row.get("published"),
        dead_lettered: row.get("dead_lettered"),
    })
}

/// Events written for a collect status change, exposed for tests and tooling.
pub fn collect_event_payload(
    collect_id: Uuid,
    status: &str,
    version: i64,
    updated_at: DateTime<Utc>,
) -> Value {
    serde_json::json!({
        "collectId": collect_id,
        "status": status,
        "version": version,
        "updatedAt": updated_at,
    })
}
