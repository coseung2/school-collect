//! Attachment metadata rules.
//!
//! Runs against a real PostgreSQL database when `DATABASE_URL` is set (the CI
//! `postgres` job and the local Docker stack both provide one) and skips
//! otherwise. Every row this test creates is removed again at the end.

use chrono::{Duration, Utc};
use school_collect_db::{
    CompleteAttachmentOutcome, CreateAttachmentOutcome, DeleteAttachmentOutcome, NewAttachment,
    NewCollect, NewCollectItem, complete_attachment, create_attachment, create_collect,
    create_tenant, delete_attachment, expired_attachments, get_attachment,
    list_collect_attachments, list_submission_attachments, purge_attachment, save_draft,
    transition_collect, upsert_user,
};
use school_collect_domain::attachments::{
    MAX_ATTACHMENTS_PER_ITEM, MAX_ATTACHMENTS_PER_SUBMISSION,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL")
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

struct Fixture {
    pool: PgPool,
    tenant: Uuid,
    admin: Uuid,
    contributor: Uuid,
    outsider: Uuid,
}

impl Fixture {
    async fn cleanup(&self) {
        let _ = sqlx::query("DELETE FROM school_collect.tenants WHERE id = $1")
            .bind(self.tenant)
            .execute(&self.pool)
            .await;
        for user in [self.admin, self.contributor, self.outsider] {
            let _ = sqlx::query("DELETE FROM school_collect.users WHERE id = $1")
                .bind(user)
                .execute(&self.pool)
                .await;
        }
    }
}

fn item(key: &str, label: &str) -> NewCollectItem {
    NewCollectItem {
        key: key.to_owned(),
        label: label.to_owned(),
        required: false,
    }
}

async fn fixture(pool: PgPool) -> Fixture {
    let run_id = Uuid::now_v7().to_string();
    let admin = upsert_user(
        &pool,
        "https://id.example.test",
        &format!("attachment-admin-{run_id}"),
        Some("admin"),
    )
    .await
    .expect("admin");
    let tenant = create_tenant(&pool, admin.id, &format!("attachment-{run_id}"))
        .await
        .expect("tenant");

    let contributor = upsert_user(
        &pool,
        "https://id.example.test",
        &format!("attachment-contributor-{run_id}"),
        Some("contributor"),
    )
    .await
    .expect("contributor");
    let outsider = upsert_user(
        &pool,
        "https://id.example.test",
        &format!("attachment-outsider-{run_id}"),
        Some("outsider"),
    )
    .await
    .expect("outsider");

    sqlx::query(
        "INSERT INTO school_collect.memberships (tenant_id, user_id, role)
         VALUES ($1, $2, 'contributor')",
    )
    .bind(tenant.id)
    .bind(contributor.id)
    .execute(&pool)
    .await
    .expect("membership");

    Fixture {
        pool,
        tenant: tenant.id,
        admin: admin.id,
        contributor: contributor.id,
        outsider: outsider.id,
    }
}

async fn new_collect(fixture: &Fixture, items: &[NewCollectItem], assignees: &[Uuid]) -> Uuid {
    create_collect(
        &fixture.pool,
        fixture.tenant,
        fixture.admin,
        &NewCollect {
            title: "attachments",
            description: "",
            due_at: None,
            items,
            assignee_ids: Some(assignees),
        },
    )
    .await
    .expect("collect")
    .id
}

async fn publish(fixture: &Fixture, collect: Uuid) {
    transition_collect(
        &fixture.pool,
        fixture.tenant,
        collect,
        &["draft"],
        "published",
        fixture.admin,
    )
    .await
    .expect("publish");
}

fn slot(tenant_id: Uuid, item_key: &str, file_name: &str, byte_size: i64) -> NewAttachment {
    let id = Uuid::now_v7();
    NewAttachment {
        id,
        item_key: item_key.to_owned(),
        file_name: file_name.to_owned(),
        content_type: "application/pdf".to_owned(),
        byte_size,
        object_key: school_collect_domain::attachments::object_key(
            &tenant_id.to_string(),
            &id.to_string(),
        ),
        expires_at: Utc::now() + Duration::days(30),
    }
}

#[tokio::test]
async fn a_target_opens_a_slot_and_the_server_completes_it() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL is not set; skipping the attachment metadata check");
        return;
    };
    let pool = school_collect_db::connect(&url).await.expect("connect");
    school_collect_db::MIGRATOR
        .run(&pool)
        .await
        .expect("migrate");
    let fixture = fixture(pool).await;
    let collect = new_collect(&fixture, &[item("plan", "계획서")], &[fixture.contributor]).await;
    publish(&fixture, collect).await;

    let attachment = slot(fixture.tenant, "plan", "계획서.pdf", 2048);
    let object_key = attachment.object_key.clone();
    let created = create_attachment(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.contributor,
        attachment.clone(),
    )
    .await
    .expect("create");
    let CreateAttachmentOutcome::Created(created) = created else {
        panic!("a target must be able to open a slot: {created:?}");
    };
    assert_eq!(created.status, "pending");
    assert_eq!(created.file_name, "계획서.pdf");
    assert_eq!(created.object_key, object_key);
    assert!(created.checksum_sha256.is_none());
    assert!(created.stored_at.is_none());

    // The declared size is a promise: a different body must not be accepted.
    let mismatch = complete_attachment(
        &fixture.pool,
        fixture.tenant,
        created.id,
        1024,
        &"a".repeat(64),
        Utc::now(),
    )
    .await
    .expect("complete");
    assert!(matches!(
        mismatch,
        CompleteAttachmentOutcome::SizeMismatch {
            declared: 2048,
            received: 1024
        }
    ));
    let still_pending = get_attachment(&fixture.pool, fixture.tenant, created.id)
        .await
        .expect("get")
        .expect("row");
    assert_eq!(still_pending.status, "pending");

    let stored = complete_attachment(
        &fixture.pool,
        fixture.tenant,
        created.id,
        2048,
        &"b".repeat(64),
        Utc::now(),
    )
    .await
    .expect("complete");
    let CompleteAttachmentOutcome::Stored(stored) = stored else {
        panic!("the same size must be accepted: {stored:?}");
    };
    assert_eq!(stored.status, "stored");
    assert_eq!(
        stored.checksum_sha256.as_deref(),
        Some("b".repeat(64).as_str())
    );
    assert!(stored.stored_at.is_some());

    // Completing twice is not an error: the second call reports what is stored.
    let again = complete_attachment(
        &fixture.pool,
        fixture.tenant,
        created.id,
        2048,
        &"b".repeat(64),
        Utc::now(),
    )
    .await
    .expect("complete");
    assert!(matches!(again, CompleteAttachmentOutcome::AlreadyStored(_)));

    let mine =
        list_submission_attachments(&fixture.pool, fixture.tenant, collect, fixture.contributor)
            .await
            .expect("list");
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].id, created.id);

    let audit: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM school_collect.audit_events
         WHERE tenant_id = $1 AND action IN ('attachment.created', 'attachment.stored')",
    )
    .bind(fixture.tenant)
    .fetch_one(&fixture.pool)
    .await
    .expect("audit count");
    assert_eq!(audit, 2, "both steps leave an audit event");

    fixture.cleanup().await;
}

#[tokio::test]
async fn only_a_target_of_an_open_collect_can_open_a_slot() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL is not set; skipping the attachment target check");
        return;
    };
    let pool = school_collect_db::connect(&url).await.expect("connect");
    school_collect_db::MIGRATOR
        .run(&pool)
        .await
        .expect("migrate");
    let fixture = fixture(pool).await;
    let collect = new_collect(&fixture, &[item("plan", "계획서")], &[fixture.contributor]).await;

    // A draft collect accepts no answers, so it accepts no attachments either.
    let attachment = slot(fixture.tenant, "plan", "계획서.pdf", 10);
    let outcome = create_attachment(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.contributor,
        attachment.clone(),
    )
    .await
    .expect("create");
    assert!(matches!(
        outcome,
        CreateAttachmentOutcome::CollectNotOpen { .. }
    ));

    publish(&fixture, collect).await;

    let outcome = create_attachment(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.outsider,
        attachment.clone(),
    )
    .await
    .expect("create");
    assert!(matches!(outcome, CreateAttachmentOutcome::NotAssigned));

    let missing_item = slot(fixture.tenant, "missing", "계획서.pdf", 10);
    let outcome = create_attachment(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.contributor,
        missing_item,
    )
    .await
    .expect("create");
    assert!(matches!(outcome, CreateAttachmentOutcome::ItemMissing));

    // An answer that was handed in keeps its files: no new slots.
    save_draft(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.contributor,
        0,
        &json!({ "plan": "제출" }),
    )
    .await
    .expect("draft");
    school_collect_db::submit(&fixture.pool, fixture.tenant, collect, fixture.contributor)
        .await
        .expect("submit");
    let outcome = create_attachment(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.contributor,
        attachment,
    )
    .await
    .expect("create");
    assert!(matches!(outcome, CreateAttachmentOutcome::AlreadySubmitted));

    fixture.cleanup().await;
}

#[tokio::test]
async fn attachment_limits_hold_per_item_and_per_submission() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL is not set; skipping the attachment limit check");
        return;
    };
    let pool = school_collect_db::connect(&url).await.expect("connect");
    school_collect_db::MIGRATOR
        .run(&pool)
        .await
        .expect("migrate");
    let fixture = fixture(pool).await;

    let items = [
        item("one", "첫째"),
        item("two", "둘째"),
        item("three", "셋째"),
        item("four", "넷째"),
        item("five", "다섯째"),
    ];
    let collect = new_collect(&fixture, &items, &[fixture.contributor]).await;
    publish(&fixture, collect).await;

    // One item fills up first.
    for index in 0..MAX_ATTACHMENTS_PER_ITEM {
        let attachment = slot(fixture.tenant, "one", &format!("계획서-{index}.pdf"), 10);
        let outcome = create_attachment(
            &fixture.pool,
            fixture.tenant,
            collect,
            fixture.contributor,
            attachment,
        )
        .await
        .expect("create");
        assert!(
            matches!(outcome, CreateAttachmentOutcome::Created(_)),
            "slot {index} must open: {outcome:?}"
        );
    }
    let overflow = slot(fixture.tenant, "one", "넘침.pdf", 10);
    let outcome = create_attachment(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.contributor,
        overflow,
    )
    .await
    .expect("create");
    assert!(matches!(outcome, CreateAttachmentOutcome::LimitReached));

    // The submission as a whole fills up next, one item at a time.
    for (key, slots) in [("two", 4), ("three", 4), ("four", 4), ("five", 3)] {
        for index in 0..slots {
            let attachment = slot(fixture.tenant, key, &format!("{key}-{index}.pdf"), 10);
            let outcome = create_attachment(
                &fixture.pool,
                fixture.tenant,
                collect,
                fixture.contributor,
                attachment,
            )
            .await
            .expect("create");
            assert!(
                matches!(outcome, CreateAttachmentOutcome::Created(_)),
                "{key} slot {index} must open: {outcome:?}"
            );
        }
    }
    let total: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM school_collect.collect_attachments
         WHERE tenant_id = $1 AND collect_id = $2 AND status <> 'deleted'",
    )
    .bind(fixture.tenant)
    .bind(collect)
    .fetch_one(&fixture.pool)
    .await
    .expect("count");
    assert_eq!(total, MAX_ATTACHMENTS_PER_SUBMISSION);

    // The last item still has room of its own, but the submission is full.
    let overflow = slot(fixture.tenant, "five", "제출-넘침.pdf", 10);
    let outcome = create_attachment(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.contributor,
        overflow,
    )
    .await
    .expect("create");
    assert!(matches!(outcome, CreateAttachmentOutcome::LimitReached));

    fixture.cleanup().await;
}

#[tokio::test]
async fn another_school_cannot_read_or_delete_an_attachment() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL is not set; skipping the attachment isolation check");
        return;
    };
    let pool = school_collect_db::connect(&url).await.expect("connect");
    school_collect_db::MIGRATOR
        .run(&pool)
        .await
        .expect("migrate");
    let fixture = fixture(pool.clone()).await;
    let collect = new_collect(&fixture, &[item("plan", "계획서")], &[fixture.contributor]).await;
    publish(&fixture, collect).await;
    let attachment = slot(fixture.tenant, "plan", "계획서.pdf", 10);
    let CreateAttachmentOutcome::Created(created) = create_attachment(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.contributor,
        attachment,
    )
    .await
    .expect("create") else {
        panic!("the target must be able to open a slot");
    };

    // A second school, with its own member.
    let other_run = Uuid::now_v7().to_string();
    let other_admin = upsert_user(
        &pool,
        "https://id.example.test",
        &format!("attachment-other-{other_run}"),
        Some("admin"),
    )
    .await
    .expect("other admin");
    let other_tenant = create_tenant(
        &pool,
        other_admin.id,
        &format!("attachment-other-{other_run}"),
    )
    .await
    .expect("other tenant");

    let read = get_attachment(&pool, other_tenant.id, created.id)
        .await
        .expect("get");
    assert!(read.is_none(), "another school must not read the row");
    let listed = list_collect_attachments(&pool, other_tenant.id, collect)
        .await
        .expect("list");
    assert!(listed.is_empty());
    let removed = delete_attachment(
        &pool,
        other_tenant.id,
        created.id,
        other_admin.id,
        false,
        Utc::now(),
    )
    .await
    .expect("delete");
    assert!(matches!(removed, DeleteAttachmentOutcome::NotFound));

    let still_there = get_attachment(&fixture.pool, fixture.tenant, created.id)
        .await
        .expect("get")
        .expect("row");
    assert_eq!(still_there.status, "pending");

    let _ = sqlx::query("DELETE FROM school_collect.tenants WHERE id = $1")
        .bind(other_tenant.id)
        .execute(&pool)
        .await;
    let _ = sqlx::query("DELETE FROM school_collect.users WHERE id = $1")
        .bind(other_admin.id)
        .execute(&pool)
        .await;
    fixture.cleanup().await;
}

#[tokio::test]
async fn deleted_and_expired_attachments_leave_the_store() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL is not set; skipping the attachment retention check");
        return;
    };
    let pool = school_collect_db::connect(&url).await.expect("connect");
    school_collect_db::MIGRATOR
        .run(&pool)
        .await
        .expect("migrate");
    let fixture = fixture(pool).await;
    let collect = new_collect(&fixture, &[item("plan", "계획서")], &[fixture.contributor]).await;
    publish(&fixture, collect).await;

    // The owner removes one file before handing the answer in.
    let attachment = slot(fixture.tenant, "plan", "지울 파일.pdf", 10);
    let object_key = attachment.object_key.clone();
    let CreateAttachmentOutcome::Created(created) = create_attachment(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.contributor,
        attachment,
    )
    .await
    .expect("create") else {
        panic!("the target must be able to open a slot");
    };
    let removed = delete_attachment(
        &fixture.pool,
        fixture.tenant,
        created.id,
        fixture.contributor,
        true,
        Utc::now(),
    )
    .await
    .expect("delete");
    let DeleteAttachmentOutcome::Deleted { object_key: key } = removed else {
        panic!("the owner must be able to remove their own file");
    };
    assert_eq!(key, object_key);
    assert!(
        get_attachment(&fixture.pool, fixture.tenant, created.id)
            .await
            .expect("get")
            .is_none()
    );
    assert!(
        list_submission_attachments(&fixture.pool, fixture.tenant, collect, fixture.contributor)
            .await
            .expect("list")
            .is_empty()
    );

    let audit: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM school_collect.audit_events
         WHERE tenant_id = $1 AND action = 'attachment.deleted'",
    )
    .bind(fixture.tenant)
    .fetch_one(&fixture.pool)
    .await
    .expect("audit count");
    assert_eq!(audit, 1);

    // An attachment whose deadline passed is offered to the sweep, and the
    // sweep removes the row once the bytes are gone.
    let mut expired = slot(fixture.tenant, "plan", "오래된 파일.pdf", 10);
    expired.expires_at = Utc::now() - Duration::days(1);
    let CreateAttachmentOutcome::Created(expired) = create_attachment(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.contributor,
        expired,
    )
    .await
    .expect("create") else {
        panic!("an expired deadline is still a valid slot");
    };
    let due = expired_attachments(&fixture.pool, Utc::now(), 100)
        .await
        .expect("expired");
    assert!(due.iter().any(|row| row.id == expired.id));

    // A file deleted before its deadline still leaves metadata behind, so it
    // is offered to the sweep once the deadline passes.
    sqlx::query(
        "UPDATE school_collect.collect_attachments SET expires_at = $3
         WHERE tenant_id = $1 AND id = $2",
    )
    .bind(fixture.tenant)
    .bind(created.id)
    .bind(Utc::now() - Duration::days(1))
    .execute(&fixture.pool)
    .await
    .expect("age the deleted row");
    let due = expired_attachments(&fixture.pool, Utc::now(), 100)
        .await
        .expect("expired");
    assert!(
        due.iter().any(|row| row.id == created.id),
        "an expired deleted row must reach the sweep"
    );
    assert!(
        purge_attachment(&fixture.pool, created.id)
            .await
            .expect("purge deleted row")
    );

    assert!(
        purge_attachment(&fixture.pool, expired.id)
            .await
            .expect("purge")
    );
    assert!(
        !purge_attachment(&fixture.pool, expired.id)
            .await
            .expect("purge again")
    );

    // Deleting the collect removes whatever is left.
    let attachment = slot(fixture.tenant, "plan", "남은 파일.pdf", 10);
    create_attachment(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.contributor,
        attachment,
    )
    .await
    .expect("create");
    sqlx::query("DELETE FROM school_collect.collects WHERE tenant_id = $1 AND id = $2")
        .bind(fixture.tenant)
        .bind(collect)
        .execute(&fixture.pool)
        .await
        .expect("delete collect");
    let remaining: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM school_collect.collect_attachments
         WHERE tenant_id = $1 AND collect_id = $2",
    )
    .bind(fixture.tenant)
    .bind(collect)
    .fetch_one(&fixture.pool)
    .await
    .expect("count");
    assert_eq!(
        remaining, 0,
        "a removed collect takes its attachments with it"
    );

    fixture.cleanup().await;
}

#[tokio::test]
async fn concurrent_requests_cannot_break_attachment_rules() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL is not set; skipping the attachment concurrency check");
        return;
    };
    let pool = school_collect_db::connect(&url).await.expect("connect");
    school_collect_db::MIGRATOR
        .run(&pool)
        .await
        .expect("migrate");
    let fixture = fixture(pool).await;
    let collect = new_collect(&fixture, &[item("plan", "계획서")], &[fixture.contributor]).await;
    publish(&fixture, collect).await;

    // Twice the per-item limit, all at once: the assignment row lock makes the
    // count-then-insert atomic, so exactly the limit succeeds.
    let attempts = (MAX_ATTACHMENTS_PER_ITEM * 2) as usize;
    let mut tasks = Vec::with_capacity(attempts);
    for index in 0..attempts {
        let pool = fixture.pool.clone();
        let (tenant, contributor) = (fixture.tenant, fixture.contributor);
        tasks.push(tokio::spawn(async move {
            create_attachment(
                &pool,
                tenant,
                collect,
                contributor,
                slot(tenant, "plan", &format!("동시-{index}.pdf"), 10),
            )
            .await
            .expect("create")
        }));
    }
    let mut created = Vec::new();
    let mut refused = 0;
    for task in tasks {
        match task.await.expect("join") {
            CreateAttachmentOutcome::Created(record) => created.push(record.id),
            CreateAttachmentOutcome::LimitReached => refused += 1,
            other => panic!("unexpected outcome: {other:?}"),
        }
    }
    assert_eq!(created.len() as i64, MAX_ATTACHMENTS_PER_ITEM);
    assert_eq!(refused, attempts - created.len());
    let stored: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM school_collect.collect_attachments
         WHERE tenant_id = $1 AND collect_id = $2 AND status <> 'deleted'",
    )
    .bind(fixture.tenant)
    .bind(collect)
    .fetch_one(&fixture.pool)
    .await
    .expect("count");
    assert_eq!(stored, MAX_ATTACHMENTS_PER_ITEM);

    // Once the answer is handed in, the owner can no longer remove its files;
    // a manager still can.
    save_draft(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.contributor,
        0,
        &json!({ "plan": "제출" }),
    )
    .await
    .expect("draft");
    school_collect_db::submit(&fixture.pool, fixture.tenant, collect, fixture.contributor)
        .await
        .expect("submit");
    let refused = delete_attachment(
        &fixture.pool,
        fixture.tenant,
        created[0],
        fixture.contributor,
        true,
        Utc::now(),
    )
    .await
    .expect("owner delete");
    assert!(matches!(refused, DeleteAttachmentOutcome::Submitted));
    let kept = get_attachment(&fixture.pool, fixture.tenant, created[0])
        .await
        .expect("get");
    assert!(
        kept.is_some(),
        "a refused delete must leave the file visible"
    );
    let removed = delete_attachment(
        &fixture.pool,
        fixture.tenant,
        created[0],
        fixture.admin,
        false,
        Utc::now(),
    )
    .await
    .expect("manager delete");
    assert!(matches!(removed, DeleteAttachmentOutcome::Deleted { .. }));

    fixture.cleanup().await;
}
