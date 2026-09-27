//! Outbox relay behavior against a real PostgreSQL database.
//!
//! Runs when `DATABASE_URL` is set (the CI `postgres` job and the local Docker
//! stack both provide one) and skips otherwise. Every row the test creates is
//! removed again at the end.

use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use school_collect_db::{
    NewCollect, NewCollectItem, OutboxFailureOutcome, OutboxStats, claim_outbox_batch,
    create_collect, insert_outbox, mark_outbox_failed, outbox_stats, transition_collect,
};
use school_collect_worker::{
    EventPublisher, PublishFailure, RelayConfig, nats::NatsPublisher, run_once,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

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

/// These tests read the shared outbox table, so they run one at a time and
/// keep their event counts exact.
static DB_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn clear_outbox(pool: &PgPool) {
    sqlx::query("DELETE FROM school_collect.outbox_events WHERE topic LIKE 'test.%'")
        .execute(pool)
        .await
        .expect("clear outbox");
}

async fn insert_test_event(pool: &PgPool, name: &str) -> Uuid {
    let mut connection = pool.acquire().await.expect("connection");
    insert_outbox(
        &mut connection,
        None,
        school_collect_db::NewOutboxEvent {
            topic: "test.events",
            event_type: name,
            aggregate_id: None,
            payload: &json!({ "name": name }),
        },
    )
    .await
    .expect("insert event")
}

#[derive(Default)]
struct RecordingPublisher {
    seen: Mutex<Vec<Uuid>>,
    fail: bool,
}

impl RecordingPublisher {
    fn failing() -> Self {
        Self {
            seen: Mutex::new(Vec::new()),
            fail: true,
        }
    }

    fn delivered(&self) -> Vec<Uuid> {
        self.seen.lock().expect("seen").clone()
    }
}

impl EventPublisher for RecordingPublisher {
    fn publish<'a>(
        &'a self,
        event: &'a school_collect_db::OutboxEventRecord,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), PublishFailure>> + Send + 'a>>
    {
        Box::pin(async move {
            if self.fail {
                return Err(PublishFailure::new("broker is unreachable"));
            }
            self.seen.lock().expect("seen").push(event.id);
            Ok(())
        })
    }
}

#[tokio::test]
async fn relay_publishes_due_events_and_never_repeats_them() {
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL is not set; skipping the outbox relay check");
        return;
    };
    let _guard = DB_LOCK.lock().await;
    clear_outbox(&pool).await;

    let first = insert_test_event(&pool, "test.first").await;
    let second = insert_test_event(&pool, "test.second").await;

    let publisher = RecordingPublisher::default();
    let report = run_once(&pool, &publisher, RelayConfig::default())
        .await
        .expect("relay round");
    assert_eq!(report.claimed, 2);
    assert_eq!(report.published, 2);
    assert_eq!(report.retried, 0);
    let delivered = publisher.delivered();
    assert!(delivered.contains(&first) && delivered.contains(&second));

    // Acknowledged events are not claimed again.
    let report = run_once(&pool, &publisher, RelayConfig::default())
        .await
        .expect("second round");
    assert_eq!(report.claimed, 0);
    assert_eq!(publisher.delivered().len(), 2);

    let stats = outbox_stats(&pool).await.expect("stats");
    assert!(stats.published >= 2);

    clear_outbox(&pool).await;
}

#[tokio::test]
async fn relay_retries_failures_then_dead_letters() {
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL is not set; skipping the outbox retry check");
        return;
    };
    let _guard = DB_LOCK.lock().await;
    clear_outbox(&pool).await;
    let event = insert_test_event(&pool, "test.failure").await;

    let publisher = RecordingPublisher::failing();
    let config = RelayConfig {
        batch_size: 10,
        max_attempts: 2,
        retry_delay: Duration::from_millis(0),
        claim_lease: Duration::from_secs(60),
    };

    let report = run_once(&pool, &publisher, config)
        .await
        .expect("first round");
    assert_eq!(report.retried, 1);
    assert_eq!(report.dead_lettered, 0);

    let report = run_once(&pool, &publisher, config)
        .await
        .expect("second round");
    assert_eq!(report.dead_lettered, 1);

    // Dead-lettered events stay visible but are no longer claimed.
    let report = run_once(&pool, &publisher, config)
        .await
        .expect("third round");
    assert_eq!(report.claimed, 0);

    let stats = outbox_stats(&pool).await.expect("stats");
    assert_eq!(stats.dead_lettered, 1);
    let error = sqlx::query_scalar::<_, Option<String>>(
        "SELECT last_error FROM school_collect.outbox_events WHERE id = $1",
    )
    .bind(event)
    .fetch_one(&pool)
    .await
    .expect("last error");
    assert!(error.unwrap_or_default().contains("unreachable"));

    clear_outbox(&pool).await;
}

#[tokio::test]
async fn concurrent_relays_never_take_the_same_event() {
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL is not set; skipping the outbox concurrency check");
        return;
    };
    let _guard = DB_LOCK.lock().await;
    clear_outbox(&pool).await;
    for index in 0..4 {
        insert_test_event(&pool, &format!("test.concurrent.{index}")).await;
    }

    let first = Arc::new(RecordingPublisher::default());
    let second = Arc::new(RecordingPublisher::default());
    let config = RelayConfig {
        batch_size: 2,
        ..RelayConfig::default()
    };

    let (left, right) = tokio::join!(
        run_once(&pool, first.as_ref(), config),
        run_once(&pool, second.as_ref(), config),
    );
    assert_eq!(
        left.expect("left").published + right.expect("right").published,
        4
    );

    let mut seen = first.delivered();
    seen.extend(second.delivered());
    seen.sort();
    let mut unique = seen.clone();
    unique.dedup();
    assert_eq!(seen.len(), 4, "every event is delivered once");
    assert_eq!(unique.len(), 4, "no event is handed to both relays");

    clear_outbox(&pool).await;
}

#[tokio::test]
async fn business_changes_write_their_event_in_the_same_transaction() {
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL is not set; skipping the outbox transaction check");
        return;
    };
    let _guard = DB_LOCK.lock().await;
    let run_id = Uuid::now_v7().to_string();
    let admin = school_collect_db::upsert_user(
        &pool,
        "https://id.example.test",
        &format!("outbox-admin-{run_id}"),
        Some("outbox-admin"),
    )
    .await
    .expect("user");
    let tenant = school_collect_db::create_tenant(&pool, admin.id, &format!("outbox-{run_id}"))
        .await
        .expect("tenant");

    let items = [NewCollectItem {
        key: "title".to_owned(),
        label: "제목".to_owned(),
        required: true,
    }];
    let collect = create_collect(
        &pool,
        tenant.id,
        admin.id,
        &NewCollect {
            title: "outbox",
            description: "",
            due_at: None,
            items: &items,
            assignee_ids: Some(&[admin.id]),
        },
    )
    .await
    .expect("collect");

    let events = sqlx::query_scalar::<_, String>(
        "SELECT event_type FROM school_collect.outbox_events
         WHERE tenant_id = $1 ORDER BY created_at",
    )
    .bind(tenant.id)
    .fetch_all(&pool)
    .await
    .expect("events");
    assert_eq!(events, vec!["collect.created"]);

    // Publishing writes its event too, in the same transaction as the status.
    transition_collect(
        &pool,
        tenant.id,
        collect.id,
        &["draft"],
        "published",
        admin.id,
    )
    .await
    .expect("publish");
    let events = sqlx::query_scalar::<_, String>(
        "SELECT event_type FROM school_collect.outbox_events
         WHERE tenant_id = $1 ORDER BY created_at",
    )
    .bind(tenant.id)
    .fetch_all(&pool)
    .await
    .expect("events");
    assert_eq!(events, vec!["collect.created", "collect.published"]);

    // A rejected transition must not leave an event behind.
    transition_collect(
        &pool,
        tenant.id,
        collect.id,
        &["draft"],
        "published",
        admin.id,
    )
    .await
    .expect("rejected transition");
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM school_collect.outbox_events
         WHERE tenant_id = $1 AND event_type = 'collect.published'",
    )
    .bind(tenant.id)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(count, 1, "the rejected transition wrote no event");

    sqlx::query("DELETE FROM school_collect.tenants WHERE id = $1")
        .bind(tenant.id)
        .execute(&pool)
        .await
        .expect("cleanup tenant");
    sqlx::query("DELETE FROM school_collect.users WHERE id = $1")
        .bind(admin.id)
        .execute(&pool)
        .await
        .expect("cleanup user");
}

/// JetStream delivery: real broker, real acknowledgement, broker-side dedup.
#[tokio::test]
async fn jetstream_acknowledges_deliveries_and_dedupes_repeats() {
    let Some(url) = std::env::var("NATS_URL")
        .ok()
        .filter(|v| !v.trim().is_empty())
    else {
        eprintln!("NATS_URL is not set; skipping the JetStream delivery check");
        return;
    };
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL is not set; skipping the JetStream delivery check");
        return;
    };
    let _guard = DB_LOCK.lock().await;
    clear_outbox(&pool).await;
    let event_id = insert_test_event(&pool, "test.jetstream").await;

    let publisher = NatsPublisher::connect(&url).await.expect("nats");
    let report = run_once(&pool, &publisher, RelayConfig::default())
        .await
        .expect("relay round");
    assert_eq!(report.published, 1);

    let published_at = sqlx::query_scalar::<_, Option<chrono::DateTime<chrono::Utc>>>(
        "SELECT published_at FROM school_collect.outbox_events WHERE id = $1",
    )
    .bind(event_id)
    .fetch_one(&pool)
    .await
    .expect("published at");
    assert!(published_at.is_some(), "the ack marked the event published");

    // The broker really stored the event body.
    let context = publisher.context().clone();
    let stream = context
        .get_stream(school_collect_worker::nats::STREAM_NAME)
        .await
        .expect("stream");
    let consumer = stream
        .create_consumer(async_nats::jetstream::consumer::pull::Config {
            deliver_policy: async_nats::jetstream::consumer::DeliverPolicy::All,
            ..Default::default()
        })
        .await
        .expect("consumer");
    let mut messages = consumer
        .fetch()
        .max_messages(16)
        .expires(Duration::from_secs(5))
        .messages()
        .await
        .expect("fetch");
    let mut stored = Vec::new();
    while let Some(message) = futures::StreamExt::next(&mut messages).await {
        let message = message.expect("message");
        stored.push(String::from_utf8_lossy(&message.payload).to_string());
    }
    assert!(
        stored
            .iter()
            .any(|body| body.contains(&event_id.to_string())),
        "the published event is in the stream"
    );

    // Publishing the same event id twice inside the duplicate window stores one
    // message, so a repeated publish after a crash cannot duplicate the event.
    let duplicate = school_collect_db::OutboxEventRecord {
        id: Uuid::now_v7(),
        tenant_id: None,
        topic: "test.events".to_owned(),
        event_type: "test.dedup".to_owned(),
        aggregate_id: None,
        payload: json!({ "name": "dedup" }),
        attempts: 1,
    };
    publisher.publish(&duplicate).await.expect("first publish");
    publisher.publish(&duplicate).await.expect("repeat publish");

    let mut messages = consumer
        .fetch()
        .max_messages(16)
        .expires(Duration::from_secs(5))
        .messages()
        .await
        .expect("fetch");
    let mut matches = 0;
    while let Some(message) = futures::StreamExt::next(&mut messages).await {
        let message = message.expect("message");
        if String::from_utf8_lossy(&message.payload).contains(&duplicate.id.to_string()) {
            matches += 1;
        }
    }
    assert_eq!(matches, 1, "the broker deduplicated the repeated publish");

    clear_outbox(&pool).await;
}

/// The relay keeps a claim counter honest even when the publish fails twice.
#[tokio::test]
async fn failed_claims_are_counted_before_publishing() {
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL is not set; skipping the outbox attempt check");
        return;
    };
    let _guard = DB_LOCK.lock().await;
    clear_outbox(&pool).await;
    let event = insert_test_event(&pool, "test.attempts").await;

    let claimed = claim_outbox_batch(&pool, 10, Duration::from_secs(60))
        .await
        .expect("claim");
    assert_eq!(claimed.len(), 1);
    assert_eq!(claimed[0].attempts, 1, "the attempt is counted on claim");

    let outcome = mark_outbox_failed(&pool, event, "boom", 5, 1_000)
        .await
        .expect("mark failed");
    assert_eq!(outcome, OutboxFailureOutcome::Retried { attempts: 1 });
    let available: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT available_at FROM school_collect.outbox_events WHERE id = $1")
            .bind(event)
            .fetch_one(&pool)
            .await
            .expect("available_at");
    assert!(
        available > chrono::Utc::now(),
        "retry is scheduled in the future"
    );

    let stats: OutboxStats = outbox_stats(&pool).await.expect("stats");
    assert!(stats.pending >= 1);

    clear_outbox(&pool).await;
}
