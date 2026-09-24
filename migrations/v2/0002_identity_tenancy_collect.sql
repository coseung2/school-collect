-- v2 identity, tenancy, and Collect tables live in a dedicated schema so the
-- retired v1 tables that still exist in `public` can never be mistaken for v2
-- objects. The runtime and the migrator both set
-- `search_path=school_collect,public`, and every object here is written with an
-- explicit schema qualifier so the migration does not depend on session state.
CREATE SCHEMA IF NOT EXISTS school_collect;

CREATE TABLE IF NOT EXISTS school_collect.tenants (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL CHECK (length(trim(name)) > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS school_collect.users (
    id UUID PRIMARY KEY,
    issuer TEXT NOT NULL,
    subject TEXT NOT NULL,
    display_name TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (issuer, subject)
);

CREATE TABLE IF NOT EXISTS school_collect.memberships (
    tenant_id UUID NOT NULL REFERENCES school_collect.tenants(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES school_collect.users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('admin', 'coordinator', 'contributor', 'viewer')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (tenant_id, user_id)
);

CREATE TABLE IF NOT EXISTS school_collect.collects (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES school_collect.tenants(id) ON DELETE CASCADE,
    title TEXT NOT NULL CHECK (length(trim(title)) > 0),
    description TEXT NOT NULL DEFAULT '',
    status TEXT NOT NULL CHECK (status IN ('draft', 'published', 'closed')),
    due_at TIMESTAMPTZ,
    version BIGINT NOT NULL DEFAULT 0 CHECK (version >= 0),
    created_by UUID NOT NULL REFERENCES school_collect.users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (tenant_id, id)
);

CREATE INDEX IF NOT EXISTS collects_tenant_status_idx
    ON school_collect.collects (tenant_id, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS school_collect.collect_items (
    id UUID PRIMARY KEY,
    collect_id UUID NOT NULL REFERENCES school_collect.collects(id) ON DELETE CASCADE,
    item_key TEXT NOT NULL CHECK (item_key ~ '^[a-z][a-z0-9_]*$'),
    label TEXT NOT NULL CHECK (length(trim(label)) > 0),
    required BOOLEAN NOT NULL DEFAULT false,
    position INTEGER NOT NULL CHECK (position >= 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (collect_id, item_key),
    UNIQUE (collect_id, position)
);

CREATE TABLE IF NOT EXISTS school_collect.collect_assignments (
    collect_id UUID NOT NULL REFERENCES school_collect.collects(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES school_collect.users(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES school_collect.tenants(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('assigned', 'started', 'submitted')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (collect_id, user_id),
    FOREIGN KEY (tenant_id, collect_id) REFERENCES school_collect.collects(tenant_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS school_collect.collect_submissions (
    id UUID PRIMARY KEY,
    collect_id UUID NOT NULL REFERENCES school_collect.collects(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES school_collect.users(id) ON DELETE CASCADE,
    tenant_id UUID NOT NULL REFERENCES school_collect.tenants(id) ON DELETE CASCADE,
    status TEXT NOT NULL CHECK (status IN ('draft', 'submitted')),
    payload JSONB NOT NULL DEFAULT '{}'::jsonb,
    version BIGINT NOT NULL DEFAULT 0 CHECK (version >= 0),
    submitted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (collect_id, user_id),
    FOREIGN KEY (tenant_id, collect_id) REFERENCES school_collect.collects(tenant_id, id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS school_collect.audit_events (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES school_collect.tenants(id) ON DELETE CASCADE,
    actor_user_id UUID REFERENCES school_collect.users(id) ON DELETE SET NULL,
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id UUID,
    request_id TEXT,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS audit_events_tenant_created_idx
    ON school_collect.audit_events (tenant_id, created_at DESC);

CREATE TABLE IF NOT EXISTS school_collect.outbox_events (
    id UUID PRIMARY KEY,
    tenant_id UUID REFERENCES school_collect.tenants(id) ON DELETE CASCADE,
    topic TEXT NOT NULL,
    event_type TEXT NOT NULL,
    aggregate_id UUID,
    payload JSONB NOT NULL,
    available_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    published_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS outbox_events_pending_idx
    ON school_collect.outbox_events (available_at, created_at)
    WHERE published_at IS NULL;
