-- One upload attempt, one object.
--
-- Before this, every writer of a slot shared the row's single object key, so an
-- upload that was taken over after the claim timeout could still write over, or
-- complete, the bytes of the attempt that replaced it. Now each attempt carries
-- its own identifier and its own object key: a late write lands on a key nobody
-- reads, and completion, release, and cleanup are rejected unless they present
-- the identifier of the attempt that currently owns the slot.

ALTER TABLE school_collect.collect_attachments
    ADD COLUMN IF NOT EXISTS upload_attempt_id UUID;

-- Bytes an attachment row no longer points at: a superseded attempt's object,
-- or one whose cleanup delete failed. The retention sweep removes them, so no
-- object can outlive the row that would have tracked it.
CREATE TABLE IF NOT EXISTS school_collect.attachment_orphans (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES school_collect.tenants(id) ON DELETE CASCADE,
    object_key TEXT NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS attachment_orphans_created_idx
    ON school_collect.attachment_orphans (created_at);
