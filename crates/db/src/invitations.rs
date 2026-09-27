use anyhow::Context;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

/// One invitation row. `code_hash` never leaves this module.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InvitationRecord {
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub email: String,
    pub role: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub accepted_at: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub enum CreateInvitationOutcome {
    Created(InvitationRecord),
    Duplicate,
}

#[derive(Debug)]
pub enum AcceptInvitationOutcome {
    Accepted { tenant_id: Uuid, role: String },
    NotFound,
    Expired,
    EmailMismatch,
    AlreadyUsed { status: String },
}

const RETURNED_COLUMNS: &str =
    "id, tenant_id, email, role, status, created_at, expires_at, accepted_at";

pub async fn create_invitation(
    pool: &PgPool,
    tenant_id: Uuid,
    invited_by: Uuid,
    email: &str,
    role: &str,
    code_hash: &str,
    expires_at: DateTime<Utc>,
) -> anyhow::Result<CreateInvitationOutcome> {
    let query = format!(
        "INSERT INTO school_collect.invitations
            (id, tenant_id, email, role, code_hash, invited_by, expires_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING {RETURNED_COLUMNS}"
    );
    match sqlx::query_as::<_, InvitationRecord>(&query)
        .bind(Uuid::now_v7())
        .bind(tenant_id)
        .bind(email)
        .bind(role)
        .bind(code_hash)
        .bind(invited_by)
        .bind(expires_at)
        .fetch_one(pool)
        .await
    {
        Ok(record) => Ok(CreateInvitationOutcome::Created(record)),
        // A pending invitation for the same address already exists: the partial
        // unique index reports 23505 instead of creating a second invite.
        Err(sqlx::Error::Database(database)) if database.code().as_deref() == Some("23505") => {
            Ok(CreateInvitationOutcome::Duplicate)
        }
        Err(error) => Err(error).context("failed to create the invitation"),
    }
}

pub async fn list_invitations(
    pool: &PgPool,
    tenant_id: Uuid,
) -> anyhow::Result<Vec<InvitationRecord>> {
    let query = format!(
        "SELECT {RETURNED_COLUMNS}
           FROM school_collect.invitations
          WHERE tenant_id = $1
          ORDER BY created_at DESC"
    );
    sqlx::query_as::<_, InvitationRecord>(&query)
        .bind(tenant_id)
        .fetch_all(pool)
        .await
        .context("failed to list invitations")
}

/// Revokes a pending invitation. Accepted or already revoked rows are returned
/// as `None` so the caller can answer 404 without exposing the audit trail.
pub async fn revoke_invitation(
    pool: &PgPool,
    tenant_id: Uuid,
    invitation_id: Uuid,
) -> anyhow::Result<Option<InvitationRecord>> {
    let query = format!(
        "UPDATE school_collect.invitations
            SET status = 'revoked'
          WHERE id = $1 AND tenant_id = $2 AND status = 'pending'
      RETURNING {RETURNED_COLUMNS}"
    );
    sqlx::query_as::<_, InvitationRecord>(&query)
        .bind(invitation_id)
        .bind(tenant_id)
        .fetch_optional(pool)
        .await
        .context("failed to revoke the invitation")
}

/// Accepts an invitation inside one transaction: the code must be pending and
/// unexpired, the verified email must match the invited address, and the
/// membership is created from the stored role rather than anything the client
/// sends. Re-accepting when a membership already exists keeps the current role.
pub async fn accept_invitation(
    pool: &PgPool,
    code_hash: &str,
    email: &str,
    user_id: Uuid,
) -> anyhow::Result<AcceptInvitationOutcome> {
    let mut transaction = pool
        .begin()
        .await
        .context("failed to open the invitation transaction")?;

    let row: Option<InvitationRecord> = sqlx::query_as(&format!(
        "SELECT {RETURNED_COLUMNS}
           FROM school_collect.invitations
          WHERE code_hash = $1
          FOR UPDATE"
    ))
    .bind(code_hash)
    .fetch_optional(&mut *transaction)
    .await
    .context("failed to read the invitation")?;

    let Some(record) = row else {
        return Ok(AcceptInvitationOutcome::NotFound);
    };

    if record.status != "pending" {
        return Ok(AcceptInvitationOutcome::AlreadyUsed {
            status: record.status,
        });
    }
    if !record.email.eq_ignore_ascii_case(email) {
        return Ok(AcceptInvitationOutcome::EmailMismatch);
    }
    if record.expires_at <= Utc::now() {
        return Ok(AcceptInvitationOutcome::Expired);
    }

    sqlx::query(
        "INSERT INTO school_collect.memberships (tenant_id, user_id, role)
         VALUES ($1, $2, $3)
         ON CONFLICT (tenant_id, user_id) DO NOTHING",
    )
    .bind(record.tenant_id)
    .bind(user_id)
    .bind(&record.role)
    .execute(&mut *transaction)
    .await
    .context("failed to create the membership")?;

    sqlx::query(
        "UPDATE school_collect.invitations
            SET status = 'accepted', accepted_at = now(), accepted_by = $2
          WHERE id = $1",
    )
    .bind(record.id)
    .bind(user_id)
    .execute(&mut *transaction)
    .await
    .context("failed to mark the invitation as accepted")?;

    // An existing membership keeps its role, so report what the row says now.
    let role: String = sqlx::query_scalar(
        "SELECT role FROM school_collect.memberships WHERE tenant_id = $1 AND user_id = $2",
    )
    .bind(record.tenant_id)
    .bind(user_id)
    .fetch_one(&mut *transaction)
    .await
    .context("failed to read the resulting membership")?;

    crate::outbox::insert_outbox(
        &mut transaction,
        Some(record.tenant_id),
        crate::outbox::NewOutboxEvent {
            topic: "school_collect.memberships",
            event_type: "membership.joined",
            aggregate_id: Some(user_id),
            payload: &serde_json::json!({
                "tenantId": record.tenant_id,
                "userId": user_id,
                "role": role,
            }),
        },
    )
    .await
    .context("failed to record the membership event")?;

    transaction
        .commit()
        .await
        .context("failed to commit the invitation acceptance")?;

    Ok(AcceptInvitationOutcome::Accepted {
        tenant_id: record.tenant_id,
        role,
    })
}
