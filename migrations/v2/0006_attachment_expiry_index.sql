-- The retention sweep now also removes rows that were deleted before they
-- expired (their metadata and any bytes left behind must not outlive the
-- retention period), so the expiry index covers every status.

DROP INDEX IF EXISTS school_collect.collect_attachments_expiry_idx;

CREATE INDEX IF NOT EXISTS collect_attachments_expires_at_idx
    ON school_collect.collect_attachments (expires_at);
