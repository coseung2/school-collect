//! Pooled-connection tenant isolation.
//!
//! Runs against a real PostgreSQL database when `DATABASE_URL` is set (the CI
//! `postgres` job and the local Docker stack both provide one) and skips
//! otherwise. Every row this test creates is removed again at the end, and the
//! assertions only look at the rows the test itself created.

use school_collect_db::{NewCollect, NewCollectItem, create_collect, create_tenant, get_collect};
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
    tenant_a: Uuid,
    tenant_b: Uuid,
    user_a: Uuid,
    user_b: Uuid,
    collect_a: Uuid,
}

impl Fixture {
    async fn cleanup(&self) {
        // Tenants cascade to memberships, collects, items, assignments,
        // submissions, invitations, audit events, and outbox events.
        for tenant in [self.tenant_a, self.tenant_b] {
            let _ = sqlx::query("DELETE FROM school_collect.tenants WHERE id = $1")
                .bind(tenant)
                .execute(&self.pool)
                .await;
        }
        for user in [self.user_a, self.user_b] {
            let _ = sqlx::query("DELETE FROM school_collect.users WHERE id = $1")
                .bind(user)
                .execute(&self.pool)
                .await;
        }
    }
}

async fn fixture(pool: PgPool) -> Fixture {
    let run_id = Uuid::now_v7().to_string();
    let user_a = school_collect_db::upsert_user(
        &pool,
        "https://id.example.test",
        &format!("isolation-a-{run_id}"),
        Some("teacher-a"),
    )
    .await
    .expect("user a");
    let user_b = school_collect_db::upsert_user(
        &pool,
        "https://id.example.test",
        &format!("isolation-b-{run_id}"),
        Some("teacher-b"),
    )
    .await
    .expect("user b");

    let tenant_a = create_tenant(&pool, user_a.id, &format!("isolation-a-{run_id}"))
        .await
        .expect("tenant a");
    let tenant_b = create_tenant(&pool, user_b.id, &format!("isolation-b-{run_id}"))
        .await
        .expect("tenant b");

    let items = [NewCollectItem {
        key: "attendance".to_owned(),
        label: "출결".to_owned(),
        required: true,
    }];
    let collect_a = create_collect(
        &pool,
        tenant_a.id,
        user_a.id,
        &NewCollect {
            title: "isolation collect",
            description: "",
            due_at: None,
            items: &items,
            assignee_ids: None,
        },
    )
    .await
    .expect("collect a");

    Fixture {
        pool,
        tenant_a: tenant_a.id,
        tenant_b: tenant_b.id,
        user_a: user_a.id,
        user_b: user_b.id,
        collect_a: collect_a.id,
    }
}

#[tokio::test]
async fn pooled_connections_never_leak_another_schools_rows() {
    let Some(url) = database_url() else {
        eprintln!("DATABASE_URL is not set; skipping the pooled tenant isolation check");
        return;
    };

    let pool = school_collect_db::connect(&url).await.expect("connect");
    school_collect_db::MIGRATOR
        .run(&pool)
        .await
        .expect("migrations apply");

    let fixture = fixture(pool).await;

    // Interleave both schools on the same pool so connections are reused.
    for _ in 0..30 {
        assert_eq!(
            school_collect_db::membership_role(&fixture.pool, fixture.tenant_a, fixture.user_a)
                .await
                .expect("role a"),
            Some("admin".to_owned())
        );
        assert_eq!(
            school_collect_db::membership_role(&fixture.pool, fixture.tenant_b, fixture.user_b)
                .await
                .expect("role b"),
            Some("admin".to_owned())
        );
        assert_eq!(
            school_collect_db::membership_role(&fixture.pool, fixture.tenant_b, fixture.user_a)
                .await
                .expect("cross role"),
            None,
            "a user of one school must never hold a role in another"
        );
        assert_eq!(
            school_collect_db::membership_role(&fixture.pool, fixture.tenant_a, fixture.user_b)
                .await
                .expect("cross role"),
            None
        );
    }

    // Memberships and collects stay inside their own school.
    let memberships = school_collect_db::list_memberships(&fixture.pool, fixture.user_a)
        .await
        .expect("memberships");
    assert!(
        memberships
            .iter()
            .any(|membership| membership.tenant_id == fixture.tenant_a)
    );
    assert!(
        !memberships
            .iter()
            .any(|membership| membership.tenant_id == fixture.tenant_b),
        "a user must not list another school's membership"
    );

    assert!(
        get_collect(&fixture.pool, fixture.tenant_a, fixture.collect_a)
            .await
            .expect("own collect")
            .is_some()
    );
    assert!(
        get_collect(&fixture.pool, fixture.tenant_b, fixture.collect_a)
            .await
            .expect("cross collect")
            .is_none(),
        "another school's collect must not be readable"
    );

    fixture.cleanup().await;
}
