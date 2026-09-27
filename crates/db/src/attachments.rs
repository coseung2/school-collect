//! Attachment metadata.
//!
//! Object bytes live in private storage; this module owns the rows that decide
//! who may read them, how long they stay, and whether an upload is still open.
//! Every query is tenant scoped, so a caller from another school finds nothing
//! to read and nothing to change.

use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use school_collect_domain::attachments::{
    MAX_ATTACHMENTS_PER_ITEM, MAX_ATTACHMENTS_PER_SUBMISSION, object_key_for_attempt,
};

/// One attachment row.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AttachmentRecord {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub collect_id: Uuid,
    pub user_id: Uuid,
    pub item_key: String,
    pub file_name: String,
    pub content_type: String,
    pub byte_size: i64,
    pub checksum_sha256: Option<String>,
    pub object_key: String,
    pub status: String,
    pub expires_at: DateTime<Utc>,
    pub stored_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    /// The upload attempt that owns the slot, or `None` before the first one.
    pub upload_attempt_id: Option<Uuid>,
}

const ATTACHMENT_COLUMNS: &str = "id, tenant_id, collect_id, user_id, item_key, file_name,
     content_type, byte_size, checksum_sha256, object_key, status, expires_at, stored_at, created_at,
     upload_attempt_id";

/// What the caller asks to store. The id and the object key are decided by the
/// server, never by the client.
#[derive(Debug, Clone)]
pub struct NewAttachment {
    pub id: Uuid,
    pub item_key: String,
    pub file_name: String,
    pub content_type: String,
    pub byte_size: i64,
    pub object_key: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug)]
pub enum CreateAttachmentOutcome {
    Created(Box<AttachmentRecord>),
    /// Missing in this school, or not accepting answers.
    CollectNotOpen {
        status: String,
    },
    ItemMissing,
    NotAssigned,
    AlreadySubmitted,
    LimitReached,
}

#[derive(Debug)]
pub enum CompleteAttachmentOutcome {
    Stored(Box<AttachmentRecord>),
    NotFound,
    AlreadyStored(Box<AttachmentRecord>),
    /// Another attempt took the slot over after the claim timeout. The bytes
    /// this writer produced belong to nobody and must not be recorded.
    ClaimLost,
    Expired,
    /// The collect closed, or the answer was handed in, while the bytes were
    /// arriving.
    NotOpen,
    SizeMismatch {
        declared: i64,
        received: i64,
    },
}

/// Result of trying to become the one writer of a slot.
#[derive(Debug)]
pub enum ClaimUploadOutcome {
    /// This request owns the slot until it completes or releases it. The record
    /// carries this attempt's identifier and object key.
    Claimed {
        record: Box<AttachmentRecord>,
        /// The object the attempt that just lost the slot may have written, if
        /// any. Its bytes belong to nobody now, so the caller removes them (or
        /// records them for the sweep) instead of leaving them untracked.
        superseded_object_key: Option<String>,
    },
    /// Another request is uploading right now.
    Busy,
    /// The bytes are already stored.
    AlreadyStored,
    NotFound,
    Expired,
    /// The collect closed, or the answer was handed in, while the slot waited.
    NotOpen,
}

/// How long a claim blocks other uploads before it counts as abandoned.
pub const UPLOAD_CLAIM_TIMEOUT_SECONDS: i64 = 300;

/// Whether this collect and this member's answer still accept new bytes.
///
/// A slot is opened while the answer is open, but the bytes can arrive later;
/// a closed collect or a handed-in answer must refuse them then, in the same
/// transaction that would otherwise accept them.
async fn attachment_accepts_content(
    tx: &mut sqlx::PgConnection,
    tenant_id: Uuid,
    collect_id: Uuid,
    user_id: Uuid,
) -> anyhow::Result<bool> {
    let collect = sqlx::query_scalar::<_, String>(
        "SELECT status FROM school_collect.collects WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(collect_id)
    .fetch_optional(&mut *tx)
    .await?;
    if collect.as_deref() != Some("published") {
        return Ok(false);
    }
    let submission = sqlx::query_scalar::<_, String>(
        "SELECT status FROM school_collect.collect_submissions
         WHERE tenant_id = $1 AND collect_id = $2 AND user_id = $3",
    )
    .bind(tenant_id)
    .bind(collect_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;
    Ok(submission.as_deref() != Some("submitted"))
}

/// Makes this request the only writer of a pending slot.
///
/// One transaction locks the row, refuses a slot that is busy, stored, gone, or
/// no longer open, then moves it to `uploading` under a fresh attempt id and a
/// fresh object key. Two concurrent uploads therefore never share an object,
/// and an upload that was taken over after the timeout cannot write where the
/// newer attempt reads.
pub async fn claim_attachment_upload(
    pool: &PgPool,
    tenant_id: Uuid,
    attachment_id: Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<ClaimUploadOutcome> {
    let mut tx = pool.begin().await?;

    let current = sqlx::query_as::<
        _,
        (
            String,
            DateTime<Utc>,
            String,
            Option<Uuid>,
            Option<DateTime<Utc>>,
            Uuid,
            Uuid,
        ),
    >(
        "SELECT status, expires_at, object_key, upload_attempt_id, upload_claimed_at,
                collect_id, user_id
         FROM school_collect.collect_attachments
         WHERE tenant_id = $1 AND id = $2
         FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(attachment_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((status, expires_at, object_key, previous_attempt, claimed_at, collect_id, user_id)) =
        current
    else {
        tx.rollback().await?;
        return Ok(ClaimUploadOutcome::NotFound);
    };
    if status == "deleted" {
        tx.rollback().await?;
        return Ok(ClaimUploadOutcome::NotFound);
    }
    if status == "stored" {
        tx.rollback().await?;
        return Ok(ClaimUploadOutcome::AlreadyStored);
    }
    if expires_at <= now {
        tx.rollback().await?;
        return Ok(ClaimUploadOutcome::Expired);
    }
    let abandoned = now - Duration::seconds(UPLOAD_CLAIM_TIMEOUT_SECONDS);
    if status == "uploading" && claimed_at.is_some_and(|at| at > abandoned) {
        tx.rollback().await?;
        return Ok(ClaimUploadOutcome::Busy);
    }
    if !attachment_accepts_content(&mut tx, tenant_id, collect_id, user_id).await? {
        tx.rollback().await?;
        return Ok(ClaimUploadOutcome::NotOpen);
    }

    let attempt_id = Uuid::now_v7();
    let attempt_key = object_key_for_attempt(
        &tenant_id.to_string(),
        &attachment_id.to_string(),
        &attempt_id.to_string(),
    );
    let claimed = sqlx::query_as::<_, AttachmentRecord>(&format!(
        "UPDATE school_collect.collect_attachments
         SET status = 'uploading', upload_claimed_at = $3, upload_attempt_id = $4,
             object_key = $5, updated_at = $3
         WHERE tenant_id = $1 AND id = $2
         RETURNING {ATTACHMENT_COLUMNS}"
    ))
    .bind(tenant_id)
    .bind(attachment_id)
    .bind(now)
    .bind(attempt_id)
    .bind(&attempt_key)
    .fetch_one(&mut *tx)
    .await?;

    // A slot that never had an attempt has no object worth removing; every
    // other previous key may hold bytes from the attempt that lost the slot.
    let superseded_object_key = previous_attempt
        .filter(|_| object_key != attempt_key)
        .map(|_| object_key);
    // Recording the superseded object in the same transaction that stops
    // pointing the row at it is what keeps it tracked: once this commits, a
    // crash before any storage call cannot lose the key. The record carries the
    // attachment it came from, so it lives exactly as long as the slot does and
    // a writer that resumes late is still covered.
    if let Some(superseded) = &superseded_object_key {
        sqlx::query(
            "INSERT INTO school_collect.attachment_orphans
               (id, tenant_id, attachment_id, object_key)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (object_key) DO NOTHING",
        )
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(attachment_id)
        .bind(superseded)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(ClaimUploadOutcome::Claimed {
        record: Box::new(claimed),
        superseded_object_key,
    })
}

/// Gives a claimed slot back after a failed upload, so the owner can retry.
///
/// Only the attempt that still owns the slot may release it: a writer whose
/// claim was taken over must not hand the slot back to `pending` while a newer
/// attempt is writing. Returns whether this attempt was still the owner.
pub async fn release_attachment_upload(
    pool: &PgPool,
    tenant_id: Uuid,
    attachment_id: Uuid,
    attempt_id: Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<bool> {
    let released = sqlx::query(
        "UPDATE school_collect.collect_attachments
         SET status = 'pending', upload_claimed_at = NULL, updated_at = $4
         WHERE tenant_id = $1 AND id = $2 AND status = 'uploading' AND upload_attempt_id = $3",
    )
    .bind(tenant_id)
    .bind(attachment_id)
    .bind(attempt_id)
    .bind(now)
    .execute(pool)
    .await?
    .rows_affected();
    Ok(released > 0)
}

#[derive(Debug)]
pub enum DeleteAttachmentOutcome {
    Deleted {
        object_key: String,
    },
    NotFound,
    /// The answer was handed in, so only a manager may remove its files.
    Submitted,
}

/// Opens one attachment slot for a submission item.
pub async fn create_attachment(
    pool: &PgPool,
    tenant_id: Uuid,
    collect_id: Uuid,
    user_id: Uuid,
    attachment: NewAttachment,
) -> anyhow::Result<CreateAttachmentOutcome> {
    let mut tx = pool.begin().await?;

    let status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM school_collect.collects WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(collect_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some(status) = status else {
        tx.rollback().await?;
        return Ok(CreateAttachmentOutcome::CollectNotOpen {
            status: "missing".to_owned(),
        });
    };
    if status != "published" {
        tx.rollback().await?;
        return Ok(CreateAttachmentOutcome::CollectNotOpen { status });
    }

    let item_exists = sqlx::query_scalar::<_, i32>(
        "SELECT 1 FROM school_collect.collect_items WHERE collect_id = $1 AND item_key = $2",
    )
    .bind(collect_id)
    .bind(&attachment.item_key)
    .fetch_optional(&mut *tx)
    .await?;
    if item_exists.is_none() {
        tx.rollback().await?;
        return Ok(CreateAttachmentOutcome::ItemMissing);
    }

    // Only a target of the collect owes an answer, and every writer of that
    // answer takes the assignment row first: locking it here is what serializes
    // slot creation against the first draft save and the submit, even when no
    // answer row exists yet (a `FOR UPDATE` on a missing row locks nothing).
    let assigned = sqlx::query_scalar::<_, i32>(
        "SELECT 1 FROM school_collect.collect_assignments
         WHERE collect_id = $1 AND user_id = $2
         FOR UPDATE",
    )
    .bind(collect_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;
    if assigned.is_none() {
        tx.rollback().await?;
        return Ok(CreateAttachmentOutcome::NotAssigned);
    }

    // An answer that was handed in keeps its files, so a slot must not open
    // after a submit. The answer row is read under the assignment lock, so a
    // submit that ran while this transaction waited is visible here.
    let submission_status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM school_collect.collect_submissions
         WHERE collect_id = $1 AND user_id = $2
         FOR UPDATE",
    )
    .bind(collect_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;
    if submission_status.as_deref() == Some("submitted") {
        tx.rollback().await?;
        return Ok(CreateAttachmentOutcome::AlreadySubmitted);
    }

    // A slot past its retention deadline can never hold bytes again, so it does
    // not use up a place: the same rule the submit check and the screen use.
    let item_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM school_collect.collect_attachments
         WHERE collect_id = $1 AND user_id = $2 AND item_key = $3
           AND status <> 'deleted' AND expires_at > now()",
    )
    .bind(collect_id)
    .bind(user_id)
    .bind(&attachment.item_key)
    .fetch_one(&mut *tx)
    .await?;
    let submission_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM school_collect.collect_attachments
         WHERE collect_id = $1 AND user_id = $2
           AND status <> 'deleted' AND expires_at > now()",
    )
    .bind(collect_id)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;
    if item_count >= MAX_ATTACHMENTS_PER_ITEM || submission_count >= MAX_ATTACHMENTS_PER_SUBMISSION
    {
        tx.rollback().await?;
        return Ok(CreateAttachmentOutcome::LimitReached);
    }

    let created = sqlx::query_as::<_, AttachmentRecord>(&format!(
        "INSERT INTO school_collect.collect_attachments
           (id, tenant_id, collect_id, user_id, item_key, file_name, content_type, byte_size,
            object_key, status, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'pending', $10)
         RETURNING {ATTACHMENT_COLUMNS}"
    ))
    .bind(attachment.id)
    .bind(tenant_id)
    .bind(collect_id)
    .bind(user_id)
    .bind(&attachment.item_key)
    .bind(&attachment.file_name)
    .bind(&attachment.content_type)
    .bind(attachment.byte_size)
    .bind(&attachment.object_key)
    .bind(attachment.expires_at)
    .fetch_one(&mut *tx)
    .await?;

    insert_audit(
        &mut tx,
        tenant_id,
        Some(user_id),
        "attachment.created",
        "collect_attachment",
        Some(created.id),
    )
    .await?;
    tx.commit().await?;
    Ok(CreateAttachmentOutcome::Created(Box::new(created)))
}

/// Marks an upload as stored once the bytes are in private storage.
///
/// The attempt identifier is required, so a writer whose claim was taken over
/// cannot complete over the bytes of the attempt that replaced it.
pub async fn complete_attachment(
    pool: &PgPool,
    tenant_id: Uuid,
    attachment_id: Uuid,
    attempt_id: Uuid,
    byte_size: i64,
    checksum_sha256: &str,
    now: DateTime<Utc>,
) -> anyhow::Result<CompleteAttachmentOutcome> {
    let mut tx = pool.begin().await?;

    let current = sqlx::query_as::<_, AttachmentRecord>(&format!(
        "SELECT {ATTACHMENT_COLUMNS} FROM school_collect.collect_attachments
         WHERE tenant_id = $1 AND id = $2
         FOR UPDATE"
    ))
    .bind(tenant_id)
    .bind(attachment_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(current) = current else {
        tx.rollback().await?;
        return Ok(CompleteAttachmentOutcome::NotFound);
    };
    if current.status == "deleted" {
        tx.rollback().await?;
        return Ok(CompleteAttachmentOutcome::NotFound);
    }
    if current.status == "stored" {
        tx.rollback().await?;
        return Ok(CompleteAttachmentOutcome::AlreadyStored(Box::new(current)));
    }
    if current.expires_at <= now {
        tx.rollback().await?;
        return Ok(CompleteAttachmentOutcome::Expired);
    }
    if current.upload_attempt_id != Some(attempt_id) {
        tx.rollback().await?;
        return Ok(CompleteAttachmentOutcome::ClaimLost);
    }
    if current.byte_size != byte_size {
        tx.rollback().await?;
        return Ok(CompleteAttachmentOutcome::SizeMismatch {
            declared: current.byte_size,
            received: byte_size,
        });
    }
    if !attachment_accepts_content(&mut tx, tenant_id, current.collect_id, current.user_id).await? {
        tx.rollback().await?;
        return Ok(CompleteAttachmentOutcome::NotOpen);
    }

    let stored = sqlx::query_as::<_, AttachmentRecord>(&format!(
        "UPDATE school_collect.collect_attachments
         SET status = 'stored', checksum_sha256 = $3, stored_at = $4, updated_at = $4,
             upload_claimed_at = NULL
         WHERE tenant_id = $1 AND id = $2
         RETURNING {ATTACHMENT_COLUMNS}"
    ))
    .bind(tenant_id)
    .bind(attachment_id)
    .bind(checksum_sha256)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;

    insert_audit(
        &mut tx,
        tenant_id,
        Some(stored.user_id),
        "attachment.stored",
        "collect_attachment",
        Some(stored.id),
    )
    .await?;
    tx.commit().await?;
    Ok(CompleteAttachmentOutcome::Stored(Box::new(stored)))
}

pub async fn get_attachment(
    pool: &PgPool,
    tenant_id: Uuid,
    attachment_id: Uuid,
) -> anyhow::Result<Option<AttachmentRecord>> {
    let record = sqlx::query_as::<_, AttachmentRecord>(&format!(
        "SELECT {ATTACHMENT_COLUMNS} FROM school_collect.collect_attachments
         WHERE tenant_id = $1 AND id = $2 AND status <> 'deleted'"
    ))
    .bind(tenant_id)
    .bind(attachment_id)
    .fetch_optional(pool)
    .await?;
    Ok(record)
}

/// Files one member attached to their own submission.
pub async fn list_submission_attachments(
    pool: &PgPool,
    tenant_id: Uuid,
    collect_id: Uuid,
    user_id: Uuid,
) -> anyhow::Result<Vec<AttachmentRecord>> {
    let records = sqlx::query_as::<_, AttachmentRecord>(&format!(
        "SELECT {ATTACHMENT_COLUMNS} FROM school_collect.collect_attachments
         WHERE tenant_id = $1 AND collect_id = $2 AND user_id = $3 AND status <> 'deleted'
         ORDER BY item_key, created_at"
    ))
    .bind(tenant_id)
    .bind(collect_id)
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(records)
}

/// Files every member attached to one collect, for the manager view.
pub async fn list_collect_attachments(
    pool: &PgPool,
    tenant_id: Uuid,
    collect_id: Uuid,
) -> anyhow::Result<Vec<AttachmentRecord>> {
    let records = sqlx::query_as::<_, AttachmentRecord>(&format!(
        "SELECT {ATTACHMENT_COLUMNS} FROM school_collect.collect_attachments
         WHERE tenant_id = $1 AND collect_id = $2 AND status <> 'deleted'
         ORDER BY user_id, item_key, created_at"
    ))
    .bind(tenant_id)
    .bind(collect_id)
    .fetch_all(pool)
    .await?;
    Ok(records)
}

/// Hides one attachment and hands back the object key to remove.
///
/// `owner_rules` applies the member rule: a handed-in answer keeps its files.
/// The submission row is locked in the same transaction, so a submit that
/// races this delete either sees the file or the file is already gone; the
/// state can never be read as "draft" and then change before the row updates.
pub async fn delete_attachment(
    pool: &PgPool,
    tenant_id: Uuid,
    attachment_id: Uuid,
    actor: Uuid,
    owner_rules: bool,
    now: DateTime<Utc>,
) -> anyhow::Result<DeleteAttachmentOutcome> {
    let mut tx = pool.begin().await?;

    // The answer row is locked before the attachment row: that is the order
    // every writer of these two tables uses, so a delete cannot deadlock with a
    // submit or an upload.
    let target = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT collect_id, user_id FROM school_collect.collect_attachments
         WHERE tenant_id = $1 AND id = $2 AND status <> 'deleted'",
    )
    .bind(tenant_id)
    .bind(attachment_id)
    .fetch_optional(&mut *tx)
    .await?;
    let Some((collect_id, owner)) = target else {
        tx.rollback().await?;
        return Ok(DeleteAttachmentOutcome::NotFound);
    };
    if owner_rules {
        let submission = sqlx::query_scalar::<_, String>(
            "SELECT status FROM school_collect.collect_submissions
             WHERE tenant_id = $1 AND collect_id = $2 AND user_id = $3
             FOR UPDATE",
        )
        .bind(tenant_id)
        .bind(collect_id)
        .bind(owner)
        .fetch_optional(&mut *tx)
        .await?;
        if submission.as_deref() == Some("submitted") {
            tx.rollback().await?;
            return Ok(DeleteAttachmentOutcome::Submitted);
        }
    }

    let deleted = sqlx::query_as::<_, AttachmentRecord>(&format!(
        "UPDATE school_collect.collect_attachments
         SET status = 'deleted', deleted_at = $3, updated_at = $3
         WHERE tenant_id = $1 AND id = $2 AND status <> 'deleted'
         RETURNING {ATTACHMENT_COLUMNS}"
    ))
    .bind(tenant_id)
    .bind(attachment_id)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(deleted) = deleted else {
        tx.rollback().await?;
        return Ok(DeleteAttachmentOutcome::NotFound);
    };

    insert_audit(
        &mut tx,
        tenant_id,
        Some(actor),
        "attachment.deleted",
        "collect_attachment",
        Some(deleted.id),
    )
    .await?;
    tx.commit().await?;
    Ok(DeleteAttachmentOutcome::Deleted {
        object_key: deleted.object_key,
    })
}

/// One attachment whose retention deadline passed.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ExpiredAttachment {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub object_key: String,
}

/// Attachments the retention sweep still has to remove.
///
/// Deleted rows are included on purpose: their bytes may still be in storage
/// if removal failed, and their metadata (file name, owner) must not outlive
/// the retention period either.
///
/// A row whose claim is still live is skipped: a writer that is actually
/// running must not have its slot (and the object it is writing) removed
/// underneath it. A claim that was abandoned stops being live after
/// [`UPLOAD_CLAIM_TIMEOUT_SECONDS`], so the next pass takes that row.
pub async fn expired_attachments(
    pool: &PgPool,
    now: DateTime<Utc>,
    limit: i64,
) -> anyhow::Result<Vec<ExpiredAttachment>> {
    let records = sqlx::query_as::<_, ExpiredAttachment>(
        "SELECT id, tenant_id, object_key FROM school_collect.collect_attachments
         WHERE expires_at <= $1
           AND (upload_claimed_at IS NULL
                OR upload_claimed_at <= $1 - make_interval(secs => $3))
         ORDER BY expires_at
         LIMIT $2",
    )
    .bind(now)
    .bind(limit)
    .bind(UPLOAD_CLAIM_TIMEOUT_SECONDS as f64)
    .fetch_all(pool)
    .await?;
    Ok(records)
}

/// Removes a row whose bytes are gone. Used by the retention sweep.
pub async fn purge_attachment(pool: &PgPool, attachment_id: Uuid) -> anyhow::Result<bool> {
    let removed = sqlx::query("DELETE FROM school_collect.collect_attachments WHERE id = $1")
        .bind(attachment_id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(removed > 0)
}

/// One object that no attachment row points at any more.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrphanAttachment {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub object_key: String,
    /// Whether the attachment this record came from is already gone. While it
    /// exists the record is kept and re-checked, because a superseded writer
    /// may still send bytes to this key.
    pub attachment_missing: bool,
}

/// Remembers bytes whose owner gave them up, so the sweep removes them.
///
/// Cleanup normally deletes the object immediately; this records it for the
/// sweep when that delete fails, or when a takeover leaves the previous
/// attempt's object behind. One row per key, so repeated failures cannot pile
/// up duplicate work.
pub async fn record_attachment_orphan(
    pool: &PgPool,
    tenant_id: Uuid,
    attachment_id: Uuid,
    object_key: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO school_collect.attachment_orphans
           (id, tenant_id, attachment_id, object_key)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (object_key) DO NOTHING",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(attachment_id)
    .bind(object_key)
    .execute(pool)
    .await?;
    Ok(())
}

/// Objects the sweep still has to remove.
///
/// Every recorded key is returned, oldest first, together with whether the
/// attachment it came from still exists. Age is not a proof that a superseded
/// writer stopped, so the record is not retired on age; it is retired once the
/// slot it belonged to is gone.
pub async fn orphaned_attachments(
    pool: &PgPool,
    limit: i64,
) -> anyhow::Result<Vec<OrphanAttachment>> {
    let records = sqlx::query_as::<_, OrphanAttachment>(
        "SELECT o.id, o.tenant_id, o.object_key, (a.id IS NULL) AS attachment_missing
         FROM school_collect.attachment_orphans o
         LEFT JOIN school_collect.collect_attachments a ON a.id = o.attachment_id
         ORDER BY o.created_at
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(records)
}

/// Forgets one orphan whose bytes are gone.
pub async fn purge_attachment_orphan(pool: &PgPool, orphan_id: Uuid) -> anyhow::Result<bool> {
    let removed = sqlx::query("DELETE FROM school_collect.attachment_orphans WHERE id = $1")
        .bind(orphan_id)
        .execute(pool)
        .await?
        .rows_affected();
    Ok(removed > 0)
}

/// Whether an attempt still owns the slot it is about to write bytes to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotOwnership {
    /// The slot is gone, or something already stored it, so there is nothing to
    /// write.
    Gone,
    /// Another attempt owns the slot now.
    Lost,
    /// This attempt still owns the slot.
    Held,
}

/// Reads the current owner of a slot.
///
/// A writer whose claim was taken over can wake up long after it stopped being
/// the owner. Asking again just before the bytes are sent means it stops
/// instead of creating an object that only a later cleanup would notice.
pub async fn attachment_slot_ownership(
    pool: &PgPool,
    tenant_id: Uuid,
    attachment_id: Uuid,
    attempt_id: Uuid,
) -> anyhow::Result<SlotOwnership> {
    let current = sqlx::query_as::<_, (String, Option<Uuid>)>(
        "SELECT status, upload_attempt_id FROM school_collect.collect_attachments
         WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(attachment_id)
    .fetch_optional(pool)
    .await?;
    Ok(match current {
        None => SlotOwnership::Gone,
        Some((status, _)) if status == "deleted" || status == "stored" => SlotOwnership::Gone,
        Some((status, owner)) if status == "uploading" && owner == Some(attempt_id) => {
            SlotOwnership::Held
        }
        Some(_) => SlotOwnership::Lost,
    })
}

async fn insert_audit(
    tx: &mut sqlx::PgConnection,
    tenant_id: Uuid,
    actor: Option<Uuid>,
    action: &str,
    resource_type: &str,
    resource_id: Option<Uuid>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO school_collect.audit_events
           (id, tenant_id, actor_user_id, action, resource_type, resource_id)
         VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(actor)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .execute(tx)
    .await?;
    Ok(())
}
