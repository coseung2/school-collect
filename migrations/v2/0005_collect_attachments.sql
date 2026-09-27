-- Attachments belong to one submission (a collect plus the member answering it)
-- and to one item key. Object bytes live in private storage; this table keeps the
-- metadata, the access policy inputs, and the retention deadline.

CREATE TABLE IF NOT EXISTS school_collect.collect_attachments (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES school_collect.tenants(id) ON DELETE CASCADE,
    collect_id UUID NOT NULL REFERENCES school_collect.collects(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES school_collect.users(id) ON DELETE CASCADE,
    item_key TEXT NOT NULL CHECK (item_key ~ '^[a-z][a-z0-9_]*$'),
    file_name TEXT NOT NULL CHECK (length(trim(file_name)) > 0 AND length(file_name) <= 120),
    content_type TEXT NOT NULL CHECK (length(trim(content_type)) > 0 AND length(content_type) <= 120),
    byte_size BIGINT NOT NULL CHECK (byte_size > 0),
    -- The server computes this while receiving the bytes, so a stored row always
    -- carries the digest of what was actually written.
    checksum_sha256 TEXT CHECK (checksum_sha256 IS NULL OR length(checksum_sha256) = 64),
    object_key TEXT NOT NULL CHECK (length(object_key) > 0),
    status TEXT NOT NULL CHECK (status IN ('pending', 'stored', 'deleted')),
    expires_at TIMESTAMPTZ NOT NULL,
    stored_at TIMESTAMPTZ,
    deleted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (status <> 'stored' OR (checksum_sha256 IS NOT NULL AND stored_at IS NOT NULL)),
    CHECK (status <> 'deleted' OR deleted_at IS NOT NULL),
    UNIQUE (tenant_id, id),
    UNIQUE (object_key),
    FOREIGN KEY (tenant_id, collect_id) REFERENCES school_collect.collects(tenant_id, id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS collect_attachments_submission_idx
    ON school_collect.collect_attachments (tenant_id, collect_id, user_id, item_key, created_at);

-- Supports the retention sweep that removes bytes whose deadline passed.
CREATE INDEX IF NOT EXISTS collect_attachments_expiry_idx
    ON school_collect.collect_attachments (expires_at)
    WHERE status <> 'deleted';
