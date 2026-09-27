-- Idempotency for delivered events.
--
-- Delivery is at-least-once: a consumer that dies between handling an event
-- and acknowledging it receives the event again after the broker's ack
-- window. This table is the key that makes handling idempotent, and a consumer
-- writes its row inside the same transaction as its effect, so a crash can
-- never apply an effect that was not recorded. It is operational bookkeeping,
-- not tenant data: no tenant, no payload, one row per handled delivery.

CREATE TABLE IF NOT EXISTS school_collect.processed_events (
    id UUID PRIMARY KEY,
    consumer TEXT NOT NULL CHECK (length(trim(consumer)) > 0),
    event_id UUID NOT NULL,
    topic TEXT NOT NULL CHECK (length(trim(topic)) > 0),
    processed_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- The idempotency key itself: a repeated delivery loses this race and
    -- leaves the existing row alone.
    UNIQUE (consumer, event_id)
);
