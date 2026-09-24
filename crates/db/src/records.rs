use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

/// Everything the API needs to describe the caller after a verified login.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserRecord {
    pub id: Uuid,
    pub issuer: String,
    pub subject: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MembershipRecord {
    pub tenant_id: Uuid,
    pub tenant_name: String,
    pub role: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TenantRecord {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CollectRecord {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub title: String,
    pub description: String,
    pub status: String,
    pub due_at: Option<DateTime<Utc>>,
    pub version: i64,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct CollectListRow {
    pub id: Uuid,
    pub title: String,
    pub status: String,
    pub due_at: Option<DateTime<Utc>>,
    pub version: i64,
    pub updated_at: DateTime<Utc>,
    pub submission_status: Option<String>,
    pub submission_version: Option<i64>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SubmissionRecord {
    pub id: Uuid,
    pub collect_id: Uuid,
    pub user_id: Uuid,
    pub status: String,
    pub payload: Value,
    pub version: i64,
    pub submitted_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

/// Result of persisting a draft, expressed as data rather than an HTTP concern.
#[derive(Debug)]
pub enum SaveDraftOutcome {
    Saved(SubmissionRecord),
    VersionConflict { current_version: i64 },
    AlreadySubmitted,
    CollectNotOpen { status: String },
}

#[derive(Debug)]
pub enum SubmitOutcome {
    Submitted(SubmissionRecord),
    AlreadySubmitted,
    NothingToSubmit,
    CollectNotOpen { status: String },
}

/// Links the verified identity to a local user row.
///
/// The issuer/subject pair comes from a verified token, so this is the only
/// place a user is created. A client can never assert its own user id.
pub async fn upsert_user(
    pool: &PgPool,
    issuer: &str,
    subject: &str,
    display_name: Option<&str>,
) -> anyhow::Result<UserRecord> {
    let record = sqlx::query_as::<_, UserRecord>(
        "INSERT INTO school_collect.users (id, issuer, subject, display_name)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (issuer, subject) DO UPDATE
           SET display_name = COALESCE(EXCLUDED.display_name, school_collect.users.display_name)
         RETURNING id, issuer, subject, display_name",
    )
    .bind(Uuid::now_v7())
    .bind(issuer)
    .bind(subject)
    .bind(display_name)
    .fetch_one(pool)
    .await?;
    Ok(record)
}

pub async fn list_memberships(
    pool: &PgPool,
    user_id: Uuid,
) -> anyhow::Result<Vec<MembershipRecord>> {
    let rows = sqlx::query_as::<_, MembershipRecord>(
        "SELECT m.tenant_id, t.name AS tenant_name, m.role
         FROM school_collect.memberships m
         JOIN school_collect.tenants t ON t.id = m.tenant_id
         WHERE m.user_id = $1
         ORDER BY t.name",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Server-side authorization lookup. Returns the role only when a membership
/// row exists, so a client-supplied tenant id can never grant access.
pub async fn membership_role(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
) -> anyhow::Result<Option<String>> {
    let role = sqlx::query_scalar::<_, String>(
        "SELECT role FROM school_collect.memberships WHERE tenant_id = $1 AND user_id = $2",
    )
    .bind(tenant_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(role)
}

pub async fn create_tenant(
    pool: &PgPool,
    user_id: Uuid,
    name: &str,
) -> anyhow::Result<TenantRecord> {
    let mut tx = pool.begin().await?;

    let tenant = sqlx::query_as::<_, TenantRecord>(
        "INSERT INTO school_collect.tenants (id, name) VALUES ($1, $2) RETURNING id, name",
    )
    .bind(Uuid::now_v7())
    .bind(name)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO school_collect.memberships (tenant_id, user_id, role)
         VALUES ($1, $2, 'admin')",
    )
    .bind(tenant.id)
    .bind(user_id)
    .execute(&mut *tx)
    .await?;

    insert_audit(
        &mut tx,
        tenant.id,
        Some(user_id),
        "tenant.created",
        "tenant",
        Some(tenant.id),
    )
    .await?;

    tx.commit().await?;
    Ok(tenant)
}

pub async fn create_collect(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
    title: &str,
    description: &str,
    due_at: Option<DateTime<Utc>>,
) -> anyhow::Result<CollectRecord> {
    let mut tx = pool.begin().await?;

    let collect = sqlx::query_as::<_, CollectRecord>(
        "INSERT INTO school_collect.collects
           (id, tenant_id, title, description, status, due_at, created_by)
         VALUES ($1, $2, $3, $4, 'draft', $5, $6)
         RETURNING id, tenant_id, title, description, status, due_at, version, updated_at",
    )
    .bind(Uuid::now_v7())
    .bind(tenant_id)
    .bind(title)
    .bind(description)
    .bind(due_at)
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await?;

    insert_audit(
        &mut tx,
        tenant_id,
        Some(user_id),
        "collect.created",
        "collect",
        Some(collect.id),
    )
    .await?;

    tx.commit().await?;
    Ok(collect)
}

pub async fn list_collects(
    pool: &PgPool,
    tenant_id: Uuid,
    user_id: Uuid,
) -> anyhow::Result<Vec<CollectListRow>> {
    let rows = sqlx::query_as::<_, CollectListRow>(
        "SELECT c.id, c.title, c.status, c.due_at, c.version, c.updated_at,
                s.status AS submission_status, s.version AS submission_version
         FROM school_collect.collects c
         LEFT JOIN school_collect.collect_submissions s
           ON s.collect_id = c.id AND s.user_id = $2
         WHERE c.tenant_id = $1
         ORDER BY c.updated_at DESC",
    )
    .bind(tenant_id)
    .bind(user_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn get_collect(
    pool: &PgPool,
    tenant_id: Uuid,
    collect_id: Uuid,
) -> anyhow::Result<Option<CollectRecord>> {
    let record = sqlx::query_as::<_, CollectRecord>(
        "SELECT id, tenant_id, title, description, status, due_at, version, updated_at
         FROM school_collect.collects WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(collect_id)
    .fetch_optional(pool)
    .await?;
    Ok(record)
}

#[derive(Debug)]
pub enum TransitionOutcome {
    Changed(CollectRecord),
    InvalidState { status: String },
    NotFound,
}

/// Applies an allowed status change and bumps the optimistic version.
///
/// The allowed source states are passed in so the domain rules stay with the
/// caller, while the row lock keeps concurrent transitions from racing.
pub async fn transition_collect(
    pool: &PgPool,
    tenant_id: Uuid,
    collect_id: Uuid,
    allowed_from: &[&str],
    to: &str,
    actor: Uuid,
) -> anyhow::Result<TransitionOutcome> {
    let mut tx = pool.begin().await?;

    let current = sqlx::query_as::<_, CollectRecord>(
        "SELECT id, tenant_id, title, description, status, due_at, version, updated_at
         FROM school_collect.collects
         WHERE tenant_id = $1 AND id = $2
         FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(collect_id)
    .fetch_optional(&mut *tx)
    .await?;

    let Some(current) = current else {
        tx.rollback().await?;
        return Ok(TransitionOutcome::NotFound);
    };

    if !allowed_from.contains(&current.status.as_str()) {
        tx.rollback().await?;
        return Ok(TransitionOutcome::InvalidState {
            status: current.status,
        });
    }

    let updated = sqlx::query_as::<_, CollectRecord>(
        "UPDATE school_collect.collects
         SET status = $3, version = version + 1, updated_at = now()
         WHERE tenant_id = $1 AND id = $2
         RETURNING id, tenant_id, title, description, status, due_at, version, updated_at",
    )
    .bind(tenant_id)
    .bind(collect_id)
    .bind(to)
    .fetch_one(&mut *tx)
    .await?;

    insert_audit(
        &mut tx,
        tenant_id,
        Some(actor),
        if to == "closed" {
            "collect.closed"
        } else {
            "collect.published"
        },
        "collect",
        Some(collect_id),
    )
    .await?;

    tx.commit().await?;
    Ok(TransitionOutcome::Changed(updated))
}

/// Persists a draft with an optimistic concurrency check.
///
/// `expected_version` is the version the client last observed. A mismatch is a
/// normal, reported conflict rather than a silent overwrite, so two devices
/// editing the same draft cannot lose each other's work.
pub async fn save_draft(
    pool: &PgPool,
    tenant_id: Uuid,
    collect_id: Uuid,
    user_id: Uuid,
    expected_version: i64,
    payload: &Value,
) -> anyhow::Result<SaveDraftOutcome> {
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
        return Ok(SaveDraftOutcome::CollectNotOpen {
            status: "missing".to_owned(),
        });
    };
    if status != "published" {
        tx.rollback().await?;
        return Ok(SaveDraftOutcome::CollectNotOpen { status });
    }

    let existing = sqlx::query_as::<_, SubmissionRecord>(
        "SELECT id, collect_id, user_id, status, payload, version, submitted_at, updated_at
         FROM school_collect.collect_submissions
         WHERE collect_id = $1 AND user_id = $2
         FOR UPDATE",
    )
    .bind(collect_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    match existing {
        None => {
            if expected_version != 0 {
                tx.rollback().await?;
                return Ok(SaveDraftOutcome::VersionConflict { current_version: 0 });
            }
            let saved = sqlx::query_as::<_, SubmissionRecord>(
                "INSERT INTO school_collect.collect_submissions
                   (id, collect_id, user_id, tenant_id, status, payload, version)
                 VALUES ($1, $2, $3, $4, 'draft', $5, 1)
                 RETURNING id, collect_id, user_id, status, payload, version, submitted_at, updated_at",
            )
            .bind(Uuid::now_v7())
            .bind(collect_id)
            .bind(user_id)
            .bind(tenant_id)
            .bind(payload)
            .fetch_one(&mut *tx)
            .await?;
            insert_audit(
                &mut tx,
                tenant_id,
                Some(user_id),
                "collect.draft_saved",
                "collect_submission",
                Some(saved.id),
            )
            .await?;
            tx.commit().await?;
            Ok(SaveDraftOutcome::Saved(saved))
        }
        Some(current) => {
            if current.status == "submitted" {
                tx.rollback().await?;
                return Ok(SaveDraftOutcome::AlreadySubmitted);
            }
            if current.version != expected_version {
                tx.rollback().await?;
                return Ok(SaveDraftOutcome::VersionConflict {
                    current_version: current.version,
                });
            }
            let saved = sqlx::query_as::<_, SubmissionRecord>(
                "UPDATE school_collect.collect_submissions
                 SET payload = $3, version = version + 1, updated_at = now()
                 WHERE collect_id = $1 AND user_id = $2
                 RETURNING id, collect_id, user_id, status, payload, version, submitted_at, updated_at",
            )
            .bind(collect_id)
            .bind(user_id)
            .bind(payload)
            .fetch_one(&mut *tx)
            .await?;
            insert_audit(
                &mut tx,
                tenant_id,
                Some(user_id),
                "collect.draft_saved",
                "collect_submission",
                Some(saved.id),
            )
            .await?;
            tx.commit().await?;
            Ok(SaveDraftOutcome::Saved(saved))
        }
    }
}

pub async fn submit(
    pool: &PgPool,
    tenant_id: Uuid,
    collect_id: Uuid,
    user_id: Uuid,
) -> anyhow::Result<SubmitOutcome> {
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
        return Ok(SubmitOutcome::CollectNotOpen {
            status: "missing".to_owned(),
        });
    };
    if status != "published" {
        tx.rollback().await?;
        return Ok(SubmitOutcome::CollectNotOpen { status });
    }

    let existing = sqlx::query_as::<_, SubmissionRecord>(
        "SELECT id, collect_id, user_id, status, payload, version, submitted_at, updated_at
         FROM school_collect.collect_submissions
         WHERE collect_id = $1 AND user_id = $2
         FOR UPDATE",
    )
    .bind(collect_id)
    .bind(user_id)
    .fetch_optional(&mut *tx)
    .await?;

    match existing {
        None => {
            tx.rollback().await?;
            Ok(SubmitOutcome::NothingToSubmit)
        }
        Some(current) if current.status == "submitted" => {
            tx.rollback().await?;
            Ok(SubmitOutcome::AlreadySubmitted)
        }
        Some(_) => {
            let submitted = sqlx::query_as::<_, SubmissionRecord>(
                "UPDATE school_collect.collect_submissions
                 SET status = 'submitted', submitted_at = now(), version = version + 1, updated_at = now()
                 WHERE collect_id = $1 AND user_id = $2
                 RETURNING id, collect_id, user_id, status, payload, version, submitted_at, updated_at",
            )
            .bind(collect_id)
            .bind(user_id)
            .fetch_one(&mut *tx)
            .await?;

            insert_audit(
                &mut tx,
                tenant_id,
                Some(user_id),
                "collect.submitted",
                "collect_submission",
                Some(submitted.id),
            )
            .await?;

            tx.commit().await?;
            Ok(SubmitOutcome::Submitted(submitted))
        }
    }
}

pub async fn get_submission(
    pool: &PgPool,
    tenant_id: Uuid,
    collect_id: Uuid,
    user_id: Uuid,
) -> anyhow::Result<Option<SubmissionRecord>> {
    let record = sqlx::query_as::<_, SubmissionRecord>(
        "SELECT id, collect_id, user_id, status, payload, version, submitted_at, updated_at
         FROM school_collect.collect_submissions
         WHERE tenant_id = $1 AND collect_id = $2 AND user_id = $3",
    )
    .bind(tenant_id)
    .bind(collect_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;
    Ok(record)
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
