//! JetStream publisher for outbox events.

use std::future::Future;

use async_nats::jetstream::{self, Context, context::PublishAckFuture};
use school_collect_db::OutboxEventRecord;
use serde_json::json;

use crate::{EventPublisher, PublishFailure};

/// Stream every outbox event is published to.
pub const STREAM_NAME: &str = "SCHOOL_COLLECT_EVENTS";
/// Subject prefix for delivered events.
pub const SUBJECT_PREFIX: &str = "school_collect.events";
/// Broker-side duplicate window; consumers still dedupe by event id.
const DUPLICATE_WINDOW: std::time::Duration = std::time::Duration::from_secs(120);

pub struct NatsPublisher {
    context: Context,
}

impl NatsPublisher {
    /// Connects and makes sure the events stream exists.
    pub async fn connect(url: &str) -> anyhow::Result<Self> {
        let client = async_nats::connect(url).await?;
        let context = jetstream::new(client);
        context
            .get_or_create_stream(jetstream::stream::Config {
                name: STREAM_NAME.to_owned(),
                subjects: vec![format!("{SUBJECT_PREFIX}.>")],
                duplicate_window: DUPLICATE_WINDOW,
                ..Default::default()
            })
            .await?;
        Ok(Self { context })
    }

    pub fn subject_for(event: &OutboxEventRecord) -> String {
        format!("{SUBJECT_PREFIX}.{}", event.event_type)
    }

    /// JetStream handle, used by consumers and tests.
    pub fn context(&self) -> &Context {
        &self.context
    }

    /// Event body sent to the broker.
    ///
    /// Only identifiers and the business payload are sent; the broker never
    /// receives cookies, tokens, or page content.
    pub fn body_for(event: &OutboxEventRecord) -> Vec<u8> {
        serde_json::to_vec(&json!({
            "id": event.id,
            "type": event.event_type,
            "topic": event.topic,
            "tenantId": event.tenant_id,
            "aggregateId": event.aggregate_id,
            "attempts": event.attempts,
            "payload": event.payload,
        }))
        .unwrap_or_default()
    }

    fn ack(
        &self,
        event: &OutboxEventRecord,
    ) -> impl Future<Output = Result<PublishAckFuture, PublishFailure>> {
        let subject = Self::subject_for(event);
        let body = Self::body_for(event);
        let mut headers = async_nats::HeaderMap::new();
        // Broker-side dedup: the same event id inside the duplicate window is
        // stored once, so a repeated publish after a crash is harmless.
        headers.insert("Nats-Msg-Id", event.id.to_string());

        let context = self.context.clone();
        async move {
            context
                .publish_with_headers(subject, headers, body.into())
                .await
                .map_err(|error| PublishFailure::new(error.to_string()))
        }
    }
}

impl EventPublisher for NatsPublisher {
    fn publish<'a>(
        &'a self,
        event: &'a OutboxEventRecord,
    ) -> std::pin::Pin<Box<dyn Future<Output = Result<(), PublishFailure>> + Send + 'a>> {
        Box::pin(async move {
            let ack = self.ack(event).await?;
            // Waiting for the acknowledgement is what makes "published" honest.
            ack.await
                .map(|_| ())
                .map_err(|error| PublishFailure::new(error.to_string()))
        })
    }
}
