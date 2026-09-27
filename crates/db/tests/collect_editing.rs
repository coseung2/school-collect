//! Item and target editing rules for a collect.
//!
//! Runs against a real PostgreSQL database when `DATABASE_URL` is set (the CI
//! `postgres` job and the local Docker stack both provide one) and skips
//! otherwise. Every row this test creates is removed again at the end.

use school_collect_db::{
    NewCollect, NewCollectItem, SaveDraftOutcome, SubmitOutcome, UpdateAssignmentsOutcome,
    UpdateItemsOutcome, create_collect, create_tenant, save_draft, submit, transition_collect,
    update_collect_assignments, update_collect_items,
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
    viewer: Uuid,
    outsider: Uuid,
}

impl Fixture {
    async fn cleanup(&self) {
        let _ = sqlx::query("DELETE FROM school_collect.tenants WHERE id = $1")
            .bind(self.tenant)
            .execute(&self.pool)
            .await;
        for user in [self.admin, self.contributor, self.viewer, self.outsider] {
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
    let admin = school_collect_db::upsert_user(
        &pool,
        "https://id.example.test",
        &format!("editing-admin-{run_id}"),
        Some("admin"),
    )
    .await
    .expect("admin");
    let tenant = create_tenant(&pool, admin.id, &format!("editing-{run_id}"))
        .await
        .expect("tenant");

    let contributor = school_collect_db::upsert_user(
        &pool,
        "https://id.example.test",
        &format!("editing-contributor-{run_id}"),
        Some("contributor"),
    )
    .await
    .expect("contributor");
    let viewer = school_collect_db::upsert_user(
        &pool,
        "https://id.example.test",
        &format!("editing-viewer-{run_id}"),
        Some("viewer"),
    )
    .await
    .expect("viewer");
    let outsider = school_collect_db::upsert_user(
        &pool,
        "https://id.example.test",
        &format!("editing-outsider-{run_id}"),
        Some("outsider"),
    )
    .await
    .expect("outsider");

    for (user, role) in [(contributor.id, "contributor"), (viewer.id, "viewer")] {
        sqlx::query(
            "INSERT INTO school_collect.memberships (tenant_id, user_id, role)
             VALUES ($1, $2, $3)",
        )
        .bind(tenant.id)
        .bind(user)
        .bind(role)
        .execute(&pool)
        .await
        .expect("membership");
    }

    Fixture {
        pool,
        tenant: tenant.id,
        admin: admin.id,
        contributor: contributor.id,
        viewer: viewer.id,
        outsider: outsider.id,
    }
}

async fn new_collect(fixture: &Fixture, items: &[NewCollectItem], assignees: &[Uuid]) -> Uuid {
    create_collect(
        &fixture.pool,
        fixture.tenant,
        fixture.admin,
        &NewCollect {
            title: "editing",
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

async fn version(fixture: &Fixture, collect: Uuid) -> i64 {
    school_collect_db::get_collect(&fixture.pool, fixture.tenant, collect)
        .await
        .expect("collect")
        .expect("collect exists")
        .version
}

#[tokio::test]
async fn items_can_be_reshaped_until_an_answer_exists() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL is not set; skipping the collect editing check");
        return;
    };
    let pool = school_collect_db::connect(&url).await.expect("connect");
    school_collect_db::MIGRATOR
        .run(&pool)
        .await
        .expect("migrate");
    let fixture = fixture(pool).await;

    let collect = new_collect(
        &fixture,
        &[item("title", "제목"), item("note", "비고")],
        &[fixture.contributor],
    )
    .await;

    // Draft: relabel, add, and remove freely.
    let outcome = update_collect_items(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.admin,
        version(&fixture, collect).await,
        &[item("title", "제목(수정)"), item("body", "본문")],
    )
    .await
    .expect("draft update");
    assert!(matches!(outcome, UpdateItemsOutcome::Updated(_)));

    let stale = version(&fixture, collect).await - 1;
    let conflict = update_collect_items(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.admin,
        stale,
        &[item("title", "제목(수정)")],
    )
    .await
    .expect("stale update");
    match conflict {
        UpdateItemsOutcome::VersionConflict { current_version } => {
            assert_eq!(current_version, version(&fixture, collect).await);
        }
        other => panic!("expected a version conflict, got {other:?}"),
    }

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

    // Published: adding and relabelling is still allowed.
    let outcome = update_collect_items(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.admin,
        version(&fixture, collect).await,
        &[
            item("title", "제목(수정)"),
            item("body", "본문"),
            item("extra", "추가"),
        ],
    )
    .await
    .expect("published update");
    assert!(matches!(outcome, UpdateItemsOutcome::Updated(_)));

    // The contributor answers the body item.
    let saved = save_draft(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.contributor,
        0,
        &json!({ "body": "답변" }),
    )
    .await
    .expect("save draft");
    assert!(matches!(saved, SaveDraftOutcome::Saved(_)));

    // Removing an answered item is refused; removing an unused one is fine.
    let blocked = update_collect_items(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.admin,
        version(&fixture, collect).await,
        &[item("title", "제목(수정)"), item("extra", "추가")],
    )
    .await
    .expect("remove answered item");
    match blocked {
        UpdateItemsOutcome::RemovalBlocked { key } => assert_eq!(key, "body"),
        other => panic!("expected a blocked removal, got {other:?}"),
    }

    let outcome = update_collect_items(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.admin,
        version(&fixture, collect).await,
        &[item("body", "본문"), item("extra", "추가")],
    )
    .await
    .expect("remove unused item");
    assert!(matches!(outcome, UpdateItemsOutcome::Updated(_)));

    transition_collect(
        &fixture.pool,
        fixture.tenant,
        collect,
        &["published"],
        "closed",
        fixture.admin,
    )
    .await
    .expect("close");
    let closed = update_collect_items(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.admin,
        version(&fixture, collect).await,
        &[item("body", "본문")],
    )
    .await
    .expect("closed update");
    assert!(matches!(closed, UpdateItemsOutcome::NotEditable { .. }));

    fixture.cleanup().await;
}

#[tokio::test]
async fn targets_follow_membership_and_saved_work() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL is not set; skipping the target editing check");
        return;
    };
    let pool = school_collect_db::connect(&url).await.expect("connect");
    school_collect_db::MIGRATOR
        .run(&pool)
        .await
        .expect("migrate");
    let fixture = fixture(pool).await;

    let collect = new_collect(
        &fixture,
        &[item("title", "제목")],
        &[fixture.contributor, fixture.admin],
    )
    .await;

    // A viewer or an outsider can never be a target.
    for user in [fixture.viewer, fixture.outsider] {
        let outcome = update_collect_assignments(
            &fixture.pool,
            fixture.tenant,
            collect,
            fixture.admin,
            version(&fixture, collect).await,
            &[fixture.contributor, user],
        )
        .await
        .expect("assign outsider");
        match outcome {
            UpdateAssignmentsOutcome::NotAssignable { user_id } => assert_eq!(user_id, user),
            other => panic!("expected a rejected target, got {other:?}"),
        }
    }

    // Narrowing to the contributor alone is allowed.
    let outcome = update_collect_assignments(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.admin,
        version(&fixture, collect).await,
        &[fixture.contributor],
    )
    .await
    .expect("narrow targets");
    assert!(matches!(outcome, UpdateAssignmentsOutcome::Updated(_)));

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

    // The admin is no longer a target, so the admin cannot save work.
    let unassigned = save_draft(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.admin,
        0,
        &json!({ "title": "관리자 입력" }),
    )
    .await
    .expect("unassigned draft");
    assert!(matches!(unassigned, SaveDraftOutcome::NotAssigned));
    let unassigned = submit(&fixture.pool, fixture.tenant, collect, fixture.admin)
        .await
        .expect("unassigned submit");
    assert!(matches!(unassigned, SubmitOutcome::NotAssigned));

    // The contributor saves work, so removing that target is refused.
    save_draft(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.contributor,
        0,
        &json!({ "title": "학생 답변" }),
    )
    .await
    .expect("contributor draft");
    let blocked = update_collect_assignments(
        &fixture.pool,
        fixture.tenant,
        collect,
        fixture.admin,
        version(&fixture, collect).await,
        &[fixture.admin],
    )
    .await
    .expect("remove answered target");
    match blocked {
        UpdateAssignmentsOutcome::RemovalBlocked { user_id } => {
            assert_eq!(user_id, fixture.contributor);
        }
        other => panic!("expected a blocked target removal, got {other:?}"),
    }

    fixture.cleanup().await;
}
