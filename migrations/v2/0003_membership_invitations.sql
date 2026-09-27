-- Membership invitations let an admin or coordinator add a teacher to their
-- school. The invite code is stored only as a SHA-256 hash; the plaintext code
-- is returned once to the inviter and passed on out of band (there is no email
-- provider in this slice). Accepting an invitation creates the membership with
-- the role recorded here, so a client can never choose its own tenant or role.
CREATE TABLE IF NOT EXISTS school_collect.invitations (
    id UUID PRIMARY KEY,
    tenant_id UUID NOT NULL REFERENCES school_collect.tenants(id) ON DELETE CASCADE,
    email TEXT NOT NULL CHECK (length(trim(email)) BETWEEN 3 AND 254),
    role TEXT NOT NULL CHECK (role IN ('contributor', 'coordinator', 'viewer')),
    code_hash TEXT NOT NULL CHECK (length(code_hash) = 64),
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'accepted', 'revoked')),
    invited_by UUID NOT NULL REFERENCES school_collect.users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    accepted_at TIMESTAMPTZ,
    accepted_by UUID REFERENCES school_collect.users(id),
    CONSTRAINT invitations_expiry_after_creation CHECK (expires_at > created_at),
    CONSTRAINT invitations_accepted_at_matches_status CHECK (
        (status = 'accepted') = (accepted_at IS NOT NULL)
    )
);

-- One pending invitation per address and school. Revoked or accepted rows stay
-- for the audit trail without blocking a later invitation.
CREATE UNIQUE INDEX IF NOT EXISTS invitations_pending_email_idx
    ON school_collect.invitations (tenant_id, lower(email))
    WHERE status = 'pending';

CREATE UNIQUE INDEX IF NOT EXISTS invitations_code_hash_idx
    ON school_collect.invitations (code_hash);

CREATE INDEX IF NOT EXISTS invitations_tenant_status_idx
    ON school_collect.invitations (tenant_id, status, created_at DESC);
