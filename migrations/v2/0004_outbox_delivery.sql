-- Transactional outbox delivery.
--
-- Business changes and their events are written in one transaction, so an
-- event can never be published for a change that rolled back. The relay marks
-- a row published only after the broker acknowledged it; repeated failures
-- move the row to a dead-letter state instead of retrying forever.
ALTER TABLE school_collect.outbox_events
    ADD COLUMN IF NOT EXISTS dead_lettered_at TIMESTAMPTZ,
    ADD COLUMN IF NOT EXISTS last_error TEXT;

-- Rows that exhausted their attempts stay visible for operators but are no
-- longer claimed by the relay.
CREATE INDEX IF NOT EXISTS outbox_events_dead_letter_idx
    ON school_collect.outbox_events (dead_lettered_at)
    WHERE dead_lettered_at IS NOT NULL;
