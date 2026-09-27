//! Consumer for the outbox events the relay publishes.
//!
//! Delivery is at-least-once. Handling is therefore idempotent: the effect and
//! the processed-events row commit in one transaction, and the broker message
//! is acknowledged only after that commit. A repeated delivery is recognised by
//! the row, repeats no effect, and is acknowledged as well so the consumer can
//! always move on. A failure returns without acknowledging, which leaves the
//! delivery pending so the broker redelivers it after its ack window.

use std::{future::Future, pin::Pin, sync::Arc, time::Duration};

use anyhow::Context as _;
use async_nats::jetstream::consumer::{AckPolicy, DeliverPolicy, PullConsumer, pull};
use async_nats::jetstream::{self, Context};
use school_collect_db::record_event_processed;
use serde_json::Value;
use sqlx::{PgConnection, PgPool};
use uuid::Uuid;

use crate::nats::{STREAM_NAME, SUBJECT_PREFIX};

/// Durable consumer this worker owns inside the events stream.
///
/// The name is also the identity written to processed events, so the record
/// that makes a delivery idempotent belongs to the consumer that handled it.
pub const CONSUMER_NAME: &str = "school_collect_worker";

/// How long one pull request waits before the broker reports that it has
/// nothing to deliver. It bounds an idle round and the shutdown wait on a quiet
/// broker; a message that arrives meanwhile is delivered immediately.
const PULL_EXPIRES: Duration = Duration::from_secs(2);

/// One event body the relay published, reduced to what handling needs.
#[derive(Debug, Clone, PartialEq)]
pub struct ConsumedEvent {
    pub id: Uuid,
    pub event_type: String,
    pub topic: String,
    /// Business payload: identifiers and status only, never personal data.
    pub payload: Value,
}

impl ConsumedEvent {
    /// Reads the body [`crate::nats::NatsPublisher::body_for`] writes.
    fn parse(body: &[u8]) -> anyhow::Result<Self> {
        let body: Value =
            serde_json::from_slice(body).context("the delivered event is not JSON")?;
        let id = body
            .get("id")
            .and_then(Value::as_str)
            .context("the delivered event has no id")?;
        let event_type = body
            .get("type")
            .and_then(Value::as_str)
            .context("the delivered event has no type")?;
        let topic = body
            .get("topic")
            .and_then(Value::as_str)
            .context("the delivered event has no topic")?;

        Ok(Self {
            id: Uuid::parse_str(id).context("the delivered event id is not a UUID")?,
            event_type: event_type.to_owned(),
            topic: topic.to_owned(),
            payload: body.get("payload").cloned().unwrap_or(Value::Null),
        })
    }
}

/// What happened to one delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConsumeOutcome {
    /// First delivery: the effect and its record committed together.
    Processed,
    /// Handled before: the effect was not repeated.
    Duplicate,
}

/// What a consumer does with one event.
///
/// The effect runs inside the transaction that records the delivery, so an
/// effect that fails leaves the event unrecorded and the broker redelivers it.
pub trait EventEffect: Send + Sync {
    fn apply<'a>(
        &'a self,
        connection: &'a mut PgConnection,
        event: &'a ConsumedEvent,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>>;
}

/// The effect this worker has today: the record of the delivery is the effect.
///
/// The outbox exists so later work (an operator notification, an export) can be
/// attached to these events; until such work exists, the consumer's honest
/// effect is that the event was handled exactly once.
#[derive(Debug, Default)]
pub struct RecordOnlyEffect;

impl EventEffect for RecordOnlyEffect {
    fn apply<'a>(
        &'a self,
        _connection: &'a mut PgConnection,
        _event: &'a ConsumedEvent,
    ) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send + 'a>> {
        Box::pin(async { Ok(()) })
    }
}

/// Handles one delivered message.
///
/// The caller may acknowledge the broker message only after this returns `Ok`:
/// the effect and the processed-events row are one commit, so a crash between
/// them cannot apply an effect that was never recorded. `Duplicate` is also an
/// acknowledgement of a delivery that was handled before — nothing was
/// repeated.
pub async fn consume_one(
    pool: &PgPool,
    consumer: &str,
    effect: &dyn EventEffect,
    message_payload: &[u8],
) -> anyhow::Result<ConsumeOutcome> {
    let event = ConsumedEvent::parse(message_payload)?;

    let mut transaction = pool.begin().await?;
    if !record_event_processed(&mut transaction, consumer, event.id, &event.topic).await? {
        // Nothing was written, so there is nothing to commit for a delivery
        // that was already handled.
        transaction.rollback().await?;
        return Ok(ConsumeOutcome::Duplicate);
    }
    effect.apply(&mut transaction, &event).await?;
    transaction.commit().await?;
    Ok(ConsumeOutcome::Processed)
}

/// A durable pull consumer over the outbox events stream.
///
/// One message is asked for at a time and acknowledged by hand, only after its
/// effect committed, so the consumer never takes on more work than it can
/// finish.
pub struct JetStreamConsumer {
    stream: jetstream::stream::Stream,
    /// Durable name, also the identity written to processed events.
    name: String,
}

impl JetStreamConsumer {
    /// Attaches to the events stream and makes sure the durable consumer exists.
    ///
    /// The stream is created when it is missing, so the relay and the consumer
    /// can start in either order without dropping an event.
    pub async fn from_context(context: Context, name: &str) -> anyhow::Result<Self> {
        let stream = context
            .get_or_create_stream(jetstream::stream::Config {
                name: STREAM_NAME.to_owned(),
                subjects: vec![format!("{SUBJECT_PREFIX}.>")],
                ..Default::default()
            })
            .await
            .context("the events stream could not be opened")?;
        stream
            .get_or_create_consumer(
                name,
                pull::Config {
                    durable_name: Some(name.to_owned()),
                    // The consumer acknowledges every delivery itself, after
                    // the transaction that records its effect.
                    ack_policy: AckPolicy::Explicit,
                    deliver_policy: DeliverPolicy::All,
                    ..Default::default()
                },
            )
            .await
            .context("the durable consumer could not be opened")?;

        Ok(Self {
            stream,
            name: name.to_owned(),
        })
    }

    /// The durable consumer this instance reads from.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Handles at most one delivery.
    ///
    /// The message is acknowledged only after the transaction that records the
    /// effect committed. A failure returns before the acknowledgement, which
    /// leaves the delivery pending for the broker to redeliver.
    pub async fn consume_next(
        &self,
        pool: &PgPool,
        effect: &dyn EventEffect,
    ) -> anyhow::Result<Option<ConsumeOutcome>> {
        let Some(message) = self.pull().await? else {
            return Ok(None);
        };
        let outcome = consume_one(pool, &self.name, effect, &message.payload).await?;
        message
            .ack()
            .await
            .map_err(|error| anyhow::anyhow!("the acknowledgement failed: {error}"))?;
        Ok(Some(outcome))
    }

    /// Requests at most one message from the broker.
    ///
    /// `None` means the broker delivered nothing inside the pull window. An
    /// idle round is not an error: the fetch stream just ends, and the caller
    /// asks again later.
    async fn pull(&self) -> anyhow::Result<Option<jetstream::Message>> {
        let consumer: PullConsumer =
            self.stream
                .get_consumer(&self.name)
                .await
                .map_err(|error| {
                    anyhow::anyhow!("the durable consumer could not be opened: {error}")
                })?;
        let mut messages = consumer
            .fetch()
            .max_messages(1)
            .expires(PULL_EXPIRES)
            .messages()
            .await
            .context("the pull request could not be sent")?;
        match futures::StreamExt::next(&mut messages).await {
            None => Ok(None),
            Some(Ok(message)) => Ok(Some(message)),
            Some(Err(error)) => anyhow::bail!("the pull reply failed: {error}"),
        }
    }
}

/// What one round of consumption did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConsumerReport {
    /// Deliveries that applied their effect for the first time.
    pub handled: usize,
    /// Deliveries of events that were handled before.
    pub duplicates: usize,
    /// Deliveries that could not be handled; the broker redelivers them.
    pub failed: usize,
    /// The error behind `failed`, so a report is diagnosable on its own.
    pub last_error: Option<String>,
}

/// Handles up to `max_messages` deliveries.
///
/// A failure ends the round: the event stays pending for redelivery, and a
/// database that just refused a write is not asked again in a tight loop.
pub async fn consume_batch(
    pool: &PgPool,
    consumer: &JetStreamConsumer,
    effect: &dyn EventEffect,
    max_messages: usize,
) -> ConsumerReport {
    let mut report = ConsumerReport::default();
    for _ in 0..max_messages {
        match consumer.consume_next(pool, effect).await {
            Ok(Some(ConsumeOutcome::Processed)) => report.handled += 1,
            Ok(Some(ConsumeOutcome::Duplicate)) => report.duplicates += 1,
            Ok(None) => break,
            Err(error) => {
                report.failed += 1;
                report.last_error = Some(format!("{error:#}"));
                tracing::error!(
                    %error,
                    "outbox event could not be handled; it stays pending for redelivery"
                );
                break;
            }
        }
    }
    report
}

/// Runs the consumer until the shutdown future resolves.
///
/// A round asks the broker for a bounded number of messages and returns at once
/// when the stream is empty, so the wait between rounds is what keeps an idle
/// consumer from spinning: `idle` after a quiet or successful round, and
/// `backoff` after a failed one, so a broker or database that just refused is
/// not hammered while it recovers.
pub async fn run_consumer_until_shutdown(
    pool: PgPool,
    consumer: Arc<JetStreamConsumer>,
    effect: Arc<dyn EventEffect>,
    batch: usize,
    idle: Duration,
    backoff: Duration,
    shutdown: impl Future<Output = ()>,
) {
    let name = consumer.name().to_owned();
    let round_name = name.clone();
    run_paced(idle, backoff, shutdown, move || {
        let pool = pool.clone();
        let consumer = consumer.clone();
        let effect = effect.clone();
        let name = round_name.clone();
        async move {
            let report = consume_batch(&pool, consumer.as_ref(), effect.as_ref(), batch).await;
            if report.handled + report.duplicates + report.failed > 0 {
                tracing::info!(
                    consumer = %name,
                    handled = report.handled,
                    duplicates = report.duplicates,
                    failed = report.failed,
                    "outbox consumer round finished"
                );
            }
            report
        }
    })
    .await;
    tracing::info!(consumer = %name, "outbox consumer stopping");
}

/// Runs `round` repeatedly, waiting `idle` after a quiet or successful round and
/// `backoff` after a failed one, until `shutdown` resolves.
async fn run_paced<F, Fut>(
    idle: Duration,
    backoff: Duration,
    shutdown: impl Future<Output = ()>,
    mut round: F,
) where
    F: FnMut() -> Fut,
    Fut: Future<Output = ConsumerReport>,
{
    tokio::pin!(shutdown);
    loop {
        let report = tokio::select! {
            () = &mut shutdown => return,
            report = round() => report,
        };
        let wait = if report.failed > 0 { backoff } else { idle };
        tokio::select! {
            () = &mut shutdown => return,
            () = tokio::time::sleep(wait) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ConsumerReport, run_paced};
    use std::{
        sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        },
        time::Duration,
    };

    /// An idle consumer must wait between rounds and a failed round must back
    /// off: the pull returns as soon as the stream is empty, so without a wait
    /// the loop would ask the broker again every round trip.
    #[tokio::test(start_paused = true)]
    async fn rounds_wait_and_a_failure_backs_off() {
        let rounds = Arc::new(AtomicUsize::new(0));
        let failures = Arc::new(AtomicUsize::new(0));
        let (stop, stopped) = tokio::sync::oneshot::channel::<()>();
        let idle = Duration::from_secs(1);
        let backoff = Duration::from_secs(5);
        let counter = rounds.clone();
        let failing = failures.clone();
        let task = tokio::spawn(run_paced(
            idle,
            backoff,
            async {
                let _ = stopped.await;
            },
            move || {
                let counter = counter.clone();
                let failing = failing.clone();
                async move {
                    counter.fetch_add(1, Ordering::SeqCst);
                    let failed = failing.fetch_add(1, Ordering::SeqCst) == 1;
                    ConsumerReport {
                        failed: usize::from(failed),
                        ..ConsumerReport::default()
                    }
                }
            },
        ));

        // The first round runs at once; an instant round must not start another.
        tokio::task::yield_now().await;
        assert_eq!(rounds.load(Ordering::SeqCst), 1);
        tokio::time::advance(idle - Duration::from_millis(1)).await;
        assert_eq!(
            rounds.load(Ordering::SeqCst),
            1,
            "an idle round waits before asking again"
        );
        tokio::time::advance(Duration::from_millis(1)).await;
        tokio::task::yield_now().await;
        assert_eq!(rounds.load(Ordering::SeqCst), 2);

        // The second round failed, so the next wait is the longer backoff.
        tokio::time::advance(backoff - Duration::from_millis(1)).await;
        assert_eq!(
            rounds.load(Ordering::SeqCst),
            2,
            "a failed round waits the backoff"
        );
        tokio::time::advance(Duration::from_millis(1)).await;
        tokio::task::yield_now().await;
        assert_eq!(rounds.load(Ordering::SeqCst), 3);

        stop.send(()).expect("stop");
        task.await.expect("the consumer loop stops on shutdown");
    }
}
