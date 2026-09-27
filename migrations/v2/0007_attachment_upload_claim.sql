-- One writer per attachment slot.
--
-- An upload first moves its slot from `pending` to `uploading` in a single
-- UPDATE, so of two concurrent uploads only one may write bytes; the other is
-- refused before touching storage. `upload_claimed_at` lets an abandoned claim
-- (a crashed request) be retaken after a timeout instead of blocking the slot.

-- Drop the old status check by what it says, not by an assumed name, so a
-- differently named constraint cannot survive and keep rejecting 'uploading'.
DO $$
DECLARE
    constraint_name TEXT;
BEGIN
    FOR constraint_name IN
        SELECT c.conname
        FROM pg_constraint c
        WHERE c.conrelid = 'school_collect.collect_attachments'::regclass
          AND c.contype = 'c'
          AND pg_get_constraintdef(c.oid) LIKE '%status%pending%stored%deleted%'
    LOOP
        EXECUTE format(
            'ALTER TABLE school_collect.collect_attachments DROP CONSTRAINT %I',
            constraint_name
        );
    END LOOP;
END
$$;

ALTER TABLE school_collect.collect_attachments
    ADD CONSTRAINT collect_attachments_status_check
    CHECK (status IN ('pending', 'uploading', 'stored', 'deleted'));

ALTER TABLE school_collect.collect_attachments
    ADD COLUMN IF NOT EXISTS upload_claimed_at TIMESTAMPTZ;
