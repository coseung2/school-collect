-- Orphan tracking lives as long as the slot it came from.
--
-- A superseded writer can still be alive: nothing in the database, the broker,
-- or the HTTP layer can prove it will never send its bytes. A fixed age cannot
-- prove that either, so the record of the object it may write is kept for as
-- long as the attachment row exists and is re-checked on every sweep pass.
-- Once the attachment itself is gone, the next pass removes the object and then
-- the record, so the object is never dropped before the row that explained it.

ALTER TABLE school_collect.attachment_orphans
    ADD COLUMN IF NOT EXISTS attachment_id UUID;

CREATE INDEX IF NOT EXISTS attachment_orphans_attachment_idx
    ON school_collect.attachment_orphans (attachment_id);
