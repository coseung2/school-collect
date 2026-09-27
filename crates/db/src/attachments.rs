//! Attachment metadata.
//!
//! Object bytes live in private storage; this module owns the rows that decide
//! who may read them, how long they stay, and whether an upload is still open.
//! Every query is tenant scoped, so a caller from another school finds nothing
//! to read and nothing to change.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use school_collect_domain::attachments::{
    MAX_ATTACHMENTS_PER_ITEM, MAX_ATTACHMENTS_PER_SUBMISSION,
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
}

const ATTACHMENT_COLUMNS: &str = "id, tenant_id, collect_id, user_id, item_key, file_name,
     content_type, byte_size, checksum_sha256, object_key, status, expires_at, stored_at, created_at";

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
    Expired,
    SizeMismatch { declared: i64, received: i64 },
}

#[derive(Debug)]
pub enum DeleteAttachmentOutcome {
    Deleted { object_key: String },
    NotFound,
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

    // Only a target of the collect owes an answer, and an answer that was handed
    // in keeps its files.
    let assigned = sqlx::query_scalar::<_, i32>(
        "SELECT 1 FROM school_collect.collect_assignments
         WHERE collect_id = $1 AND user_id = $2",
    )
    .bind(collect_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;
    if assigned.is_none() {
        tx.rollback().await?;
        return Ok(CreateAttachmentOutcome::NotAssigned);
    }

    let submission_status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM school_collect.collect_submissions
         WHERE collect_id = $1 AND user_id = $2",
    )
    .bind(collect_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;
    if submission_status.as_deref() == Some("submitted") {
        tx.rollback().await?;
        return Ok(CreateAttachmentOutcome::AlreadySubmitted);
    }

    let item_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM school_collect.collect_attachments
         WHERE collect_id = $1 AND user_id = $2 AND item_key = $3 AND status <> 'deleted'",
    )
    .bind(collect_id)
    .bind(user_id)
    .bind(&attachment.item_key)
    .fetch_one(&mut *tx)
    .await?;
    let submission_count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM school_collect.collect_attachments
         WHERE collect_id = $1 AND user_id = $2 AND status <> 'deleted'",
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
pub async fn complete_attachment(
    pool: &PgPool,
    tenant_id: Uuid,
    attachment_id: Uuid,
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
    if current.byte_size != byte_size {
        tx.rollback().await?;
        return Ok(CompleteAttachmentOutcome::SizeMismatch {
            declared: current.byte_size,
            received: byte_size,
        });
    }

    let stored = sqlx::query_as::<_, AttachmentRecord>(&format!(
        "UPDATE school_collect.collect_attachments
         SET status = 'stored', checksum_sha256 = $3, stored_at = $4, updated_at = $4
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
pub async fn delete_attachment(
    pool: &PgPool,
    tenant_id: Uuid,
    attachment_id: Uuid,
    actor: Uuid,
    now: DateTime<Utc>,
) -> anyhow::Result<DeleteAttachmentOutcome> {
    let mut tx = pool.begin().await?;

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
pub async fn expired_attachments(
    pool: &PgPool,
    now: DateTime<Utc>,
    limit: i64,
) -> anyhow::Result<Vec<ExpiredAttachment>> {
    let records = sqlx::query_as::<_, ExpiredAttachment>(
        "SELECT id, tenant_id, object_key FROM school_collect.collect_attachments
         WHERE status <> 'deleted' AND expires_at <= $1
         ORDER BY expires_at
         LIMIT $2",
    )
    .bind(now)
    .bind(limit)
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
