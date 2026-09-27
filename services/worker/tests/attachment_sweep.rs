//! Attachment retention sweep against a real PostgreSQL database.
//!
//! Runs when `DATABASE_URL` is set (the CI `postgres` job and the local Docker
//! stack both provide one) and skips otherwise. Every row and file the test
//! creates is removed again at the end, including when a check fails.

use std::{sync::Arc, time::Duration as StdDuration};

use chrono::{Duration, Utc};
use school_collect_application::storage::{
    FileStorage, ObjectStorage, StorageError, StorageFuture,
};
use school_collect_db::{
    NewAttachment, NewCollect, NewCollectItem, create_attachment, create_collect, create_tenant,
    record_attachment_orphan, transition_collect, upsert_user,
};
use school_collect_worker::{purge_expired_attachments, run_sweep_until_shutdown};
use sqlx::PgPool;
use uuid::Uuid;

/// Storage that always refuses, so a failed sweep can be observed.
struct RefusingStorage;

impl ObjectStorage for RefusingStorage {
    fn put<'a>(
        &'a self,
        _key: &'a str,
        _bytes: Vec<u8>,
        _content_type: &'a str,
    ) -> StorageFuture<'a, Result<(), StorageError>> {
        Box::pin(async { Err(StorageError::Failed("refused".to_owned())) })
    }

    fn get<'a>(
        &'a self,
        _key: &'a str,
    ) -> StorageFuture<'a, Result<Option<Vec<u8>>, StorageError>> {
        Box::pin(async { Err(StorageError::Failed("refused".to_owned())) })
    }

    fn delete<'a>(&'a self, _key: &'a str) -> StorageFuture<'a, Result<(), StorageError>> {
        Box::pin(async { Err(StorageError::Failed("refused".to_owned())) })
    }
}

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

async fn pool() -> Option<PgPool> {
    let url = database_url()?;
    let pool = school_collect_db::connect(&url).await.expect("connect");
    school_collect_db::MIGRATOR
        .run(&pool)
        .await
        .expect("migrate");
    Some(pool)
}

struct Fixture {
    pool: PgPool,
    tenant: Uuid,
    owner: Uuid,
    author: Uuid,
}

impl Fixture {
    async fn cleanup(&self) {
        let _ = sqlx::query("DELETE FROM school_collect.tenants WHERE id = $1")
            .bind(self.tenant)
            .execute(&self.pool)
            .await;
        for user in [self.owner, self.author] {
            let _ = sqlx::query("DELETE FROM school_collect.users WHERE id = $1")
                .bind(user)
                .execute(&self.pool)
                .await;
        }
    }
}

async fn fixture(pool: PgPool) -> Fixture {
    let run_id = Uuid::now_v7().to_string();
    let owner = upsert_user(
        &pool,
        "https://id.example.test",
        &format!("sweep-owner-{run_id}"),
        Some("admin"),
    )
    .await
    .expect("owner");
    let tenant = create_tenant(&pool, owner.id, &format!("sweep-{run_id}"))
        .await
        .expect("tenant");
    let author = upsert_user(
        &pool,
        "https://id.example.test",
        &format!("sweep-author-{run_id}"),
        Some("contributor"),
    )
    .await
    .expect("author");
    sqlx::query(
        "INSERT INTO school_collect.memberships (tenant_id, user_id, role)
         VALUES ($1, $2, 'contributor')",
    )
    .bind(tenant.id)
    .bind(author.id)
    .execute(&pool)
    .await
    .expect("membership");

    Fixture {
        pool,
        tenant: tenant.id,
        owner: owner.id,
        author: author.id,
    }
}

/// Publishes one collect with a single item and opens an attachment slot on it.
async fn collect_with_slot(
    fixture: &Fixture,
    storage: &FileStorage,
    expires_in: Duration,
) -> Result<(Uuid, String), String> {
    let collect = create_collect(
        &fixture.pool,
        fixture.tenant,
        fixture.owner,
        &NewCollect {
            title: "sweep",
            description: "",
            due_at: None,
            items: &[NewCollectItem {
                key: "plan".to_owned(),
                label: "계획서".to_owned(),
                required: false,
            }],
            assignee_ids: Some(&[fixture.author]),
        },
    )
    .await
    .map_err(|error| format!("collect failed: {error}"))?
    .id;
    transition_collect(
        &fixture.pool,
        fixture.tenant,
        collect,
        &["draft"],
        "published",
        fixture.owner,
    )
    .await
    .map_err(|error| format!("publish failed: {error}"))?;

    let id = Uuid::now_v7();
    let object_key = school_collect_domain::attachments::object_key(
        &fixture.tenant.to_string(),
        &id.to_string(),
    );
    let created = create_attachment(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.author,
        NewAttachment {
            id,
            item_key: "plan".to_owned(),
            file_name: "오래된 파일.pdf".to_owned(),
            content_type: "application/pdf".to_owned(),
            byte_size: 5,
            object_key: object_key.clone(),
            expires_at: Utc::now() + expires_in,
        },
    )
    .await
    .map_err(|error| format!("attachment failed: {error}"))?;
    if !matches!(
        created,
        school_collect_db::CreateAttachmentOutcome::Created(_)
    ) {
        return Err(format!("the slot must open: {created:?}"));
    }
    storage
        .put(&object_key, b"bytes".to_vec(), "application/pdf")
        .await
        .map_err(|error| format!("storing bytes failed: {error}"))?;
    Ok((id, object_key))
}

#[tokio::test]
async fn the_sweep_removes_expired_attachments_and_keeps_the_rest() {
    let Some(pool) = pool().await else {
        eprintln!("DATABASE_URL is not set; skipping the attachment sweep check");
        return;
    };
    let fixture = fixture(pool).await;
    let root = std::env::temp_dir().join(format!("attachment-sweep-{}", Uuid::now_v7()));
    let _ = std::fs::remove_dir_all(&root);
    let storage = FileStorage::new(root.clone());

    let result: Result<(), String> = async {
        let (expired_id, expired_key) =
            collect_with_slot(&fixture, &storage, Duration::hours(-1)).await?;
        let (live_id, live_key) = collect_with_slot(&fixture, &storage, Duration::hours(1)).await?;

        let report = purge_expired_attachments(&fixture.pool, &storage, 100)
            .await
            .map_err(|error| format!("sweep failed: {error}"))?;
        if report.considered < 1 || report.purged < 1 || report.failed != 0 {
            return Err(format!("the sweep report is wrong: {report:?}"));
        }

        let remaining: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM school_collect.collect_attachments
             WHERE tenant_id = $1 AND id = ANY($2)",
        )
        .bind(fixture.tenant)
        .bind(vec![expired_id, live_id])
        .fetch_one(&fixture.pool)
        .await
        .map_err(|error| format!("count failed: {error}"))?;
        if remaining != 1 {
            return Err(format!("only the live attachment may stay: {remaining}"));
        }
        if storage
            .get(&expired_key)
            .await
            .map_err(|error| format!("get failed: {error}"))?
            .is_some()
        {
            return Err("the expired bytes must be gone".to_owned());
        }
        if storage
            .get(&live_key)
            .await
            .map_err(|error| format!("get failed: {error}"))?
            != Some(b"bytes".to_vec())
        {
            return Err("the live bytes must stay".to_owned());
        }

        // A storage failure must not delete the row: the next pass retries.
        let (retry_id, _) = collect_with_slot(&fixture, &storage, Duration::hours(-1)).await?;
        let report = purge_expired_attachments(&fixture.pool, &RefusingStorage, 100)
            .await
            .map_err(|error| format!("sweep with a refusing store failed: {error}"))?;
        if report.failed < 1 {
            return Err(format!("a refusing store must be reported: {report:?}"));
        }
        let still_there: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM school_collect.collect_attachments
             WHERE tenant_id = $1 AND id = $2",
        )
        .bind(fixture.tenant)
        .bind(retry_id)
        .fetch_one(&fixture.pool)
        .await
        .map_err(|error| format!("count failed: {error}"))?;
        if still_there != 1 {
            return Err("a storage failure keeps the row for a retry".to_owned());
        }

        // A record for a slot that still exists is kept: age cannot prove the
        // writer that lost the slot stopped, so the object is deleted again on
        // every pass and the record is retired only once the slot is gone.
        let kept_key = format!(
            "tenants/{}/attachments/{}/kept",
            fixture.tenant,
            Uuid::now_v7()
        );
        storage
            .put(&kept_key, b"kept".to_vec(), "application/pdf")
            .await
            .map_err(|error| format!("orphan put failed: {error}"))?;
        record_attachment_orphan(&fixture.pool, fixture.tenant, live_id, &kept_key)
            .await
            .map_err(|error| format!("orphan record failed: {error}"))?;
        let report = purge_expired_attachments(&fixture.pool, &storage, 100)
            .await
            .map_err(|error| format!("sweep failed: {error}"))?;
        if report.failed != 0 {
            return Err(format!("the sweep failed: {report:?}"));
        }
        if report.orphans_kept < 1 {
            return Err(format!("a record for a live slot must be kept: {report:?}"));
        }
        let kept: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM school_collect.attachment_orphans WHERE object_key = $1",
        )
        .bind(&kept_key)
        .fetch_one(&fixture.pool)
        .await
        .map_err(|error| format!("count failed: {error}"))?;
        if kept != 1 {
            return Err("the record must outlive the first cleanup pass".to_owned());
        }
        if storage
            .get(&kept_key)
            .await
            .map_err(|error| format!("get failed: {error}"))?
            .is_some()
        {
            return Err("the object must be removed on the pass that keeps the record".to_owned());
        }

        // Once the slot is gone, the next pass removes the record as well, so a
        // late write to that key is still deleted before the record is retired.
        sqlx::query("DELETE FROM school_collect.collect_attachments WHERE id = $1")
            .bind(live_id)
            .execute(&fixture.pool)
            .await
            .map_err(|error| format!("slot delete failed: {error}"))?;
        let report = purge_expired_attachments(&fixture.pool, &storage, 100)
            .await
            .map_err(|error| format!("sweep failed: {error}"))?;
        if report.orphans_removed < 1 || report.failed != 0 {
            return Err(format!("the retired record was not removed: {report:?}"));
        }
        let kept: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM school_collect.attachment_orphans WHERE object_key = $1",
        )
        .bind(&kept_key)
        .fetch_one(&fixture.pool)
        .await
        .map_err(|error| format!("count failed: {error}"))?;
        if kept != 0 {
            return Err(format!("the record must be gone: {kept}"));
        }

        // A storage failure keeps the record for the next pass.
        let (stubborn_attachment, _) =
            collect_with_slot(&fixture, &storage, Duration::hours(1)).await?;
        let stubborn_key = format!(
            "tenants/{}/attachments/{}/stubborn",
            fixture.tenant,
            Uuid::now_v7()
        );
        record_attachment_orphan(
            &fixture.pool,
            fixture.tenant,
            stubborn_attachment,
            &stubborn_key,
        )
        .await
        .map_err(|error| format!("orphan record failed: {error}"))?;
        let report = purge_expired_attachments(&fixture.pool, &RefusingStorage, 100)
            .await
            .map_err(|error| format!("sweep with a refusing store failed: {error}"))?;
        if report.failed < 1 {
            return Err(format!("a refusing store must be reported: {report:?}"));
        }
        let stubborn: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM school_collect.attachment_orphans WHERE object_key = $1",
        )
        .bind(&stubborn_key)
        .fetch_one(&fixture.pool)
        .await
        .map_err(|error| format!("count failed: {error}"))?;
        if stubborn != 1 {
            return Err(format!("a refusing store keeps the record: {stubborn}"));
        }

        // A slot whose claim is still live is not swept, even after its
        // deadline passed: a writer that is actually running must not have its
        // object deleted underneath it.
        let (claimed_id, claimed_key) =
            collect_with_slot(&fixture, &storage, Duration::hours(1)).await?;
        sqlx::query(
            "UPDATE school_collect.collect_attachments
             SET status = 'uploading', upload_claimed_at = now(), upload_attempt_id = $2,
                 expires_at = now() - interval '1 hour'
             WHERE id = $1",
        )
        .bind(claimed_id)
        .bind(Uuid::now_v7())
        .execute(&fixture.pool)
        .await
        .map_err(|error| format!("claim failed: {error}"))?;
        let report = purge_expired_attachments(&fixture.pool, &storage, 100)
            .await
            .map_err(|error| format!("sweep failed: {error}"))?;
        if report.failed != 0 {
            return Err(format!("the sweep failed: {report:?}"));
        }
        let alive: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM school_collect.collect_attachments WHERE id = $1",
        )
        .bind(claimed_id)
        .fetch_one(&fixture.pool)
        .await
        .map_err(|error| format!("count failed: {error}"))?;
        if alive != 1 {
            return Err("a live claim must keep its row".to_owned());
        }
        if storage
            .get(&claimed_key)
            .await
            .map_err(|error| format!("get failed: {error}"))?
            .is_none()
        {
            return Err("a live claim must keep its bytes".to_owned());
        }

        // Once the claim ages out, the same slot is taken.
        sqlx::query(
            "UPDATE school_collect.collect_attachments
             SET upload_claimed_at = now() - interval '1 hour'
             WHERE id = $1",
        )
        .bind(claimed_id)
        .execute(&fixture.pool)
        .await
        .map_err(|error| format!("age failed: {error}"))?;
        let report = purge_expired_attachments(&fixture.pool, &storage, 100)
            .await
            .map_err(|error| format!("sweep failed: {error}"))?;
        if report.failed != 0 {
            return Err(format!("the sweep failed: {report:?}"));
        }
        let alive: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM school_collect.collect_attachments WHERE id = $1",
        )
        .bind(claimed_id)
        .fetch_one(&fixture.pool)
        .await
        .map_err(|error| format!("count failed: {error}"))?;
        if alive != 0 {
            return Err("an abandoned claim must not block the sweep".to_owned());
        }
        if storage
            .get(&claimed_key)
            .await
            .map_err(|error| format!("get failed: {error}"))?
            .is_some()
        {
            return Err("the abandoned slot's bytes must be gone".to_owned());
        }

        // The long-running loop stops when the shutdown future resolves.
        let stopping = Arc::new(tokio::sync::Notify::new());
        let shutdown = {
            let stopping = stopping.clone();
            async move {
                stopping.notified().await;
            }
        };
        let sweep = tokio::spawn(run_sweep_until_shutdown(
            fixture.pool.clone(),
            Arc::new(FileStorage::new(root.clone())),
            100,
            StdDuration::from_millis(50),
            shutdown,
        ));
        tokio::time::sleep(StdDuration::from_millis(150)).await;
        stopping.notify_one();
        tokio::time::timeout(StdDuration::from_secs(5), sweep)
            .await
            .map_err(|_| "the sweep loop must stop on shutdown".to_owned())?
            .map_err(|error| format!("the sweep task failed: {error}"))?;

        Ok(())
    }
    .await;

    fixture.cleanup().await;
    let _ = std::fs::remove_dir_all(&root);

    if let Err(message) = result {
        panic!("attachment sweep verification failed: {message}");
    }
}
