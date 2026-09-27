//! End-to-end verification of the authenticated Collect flow.
//!
//! The test talks to the real identity provider and a real PostgreSQL database,
//! so it only runs when the documented environment variables are present and
//! skips silently otherwise. Every row and identity it creates is removed again
//! before the test returns, including on failure.
//!
//! Required environment:
//!   DATABASE_URL                 - PostgreSQL connection string (v2 migrator applies)
//!   SUPABASE_URL                 - https://<project-ref>.supabase.co
//!   SUPABASE_ANON_KEY            - public anon key used by the client
//!   SUPABASE_SERVICE_ROLE_KEY    - admin key used only to create/delete the test identity

use axum::{
    Router,
    body::Body,
    http::{HeaderValue, Request, StatusCode, header},
};
use school_collect_api::{AppState, AuthState, router};
use school_collect_auth::{OidcConfig, OidcVerifier};
use serde_json::{Value, json};
use sqlx::{PgPool, postgres::PgPoolOptions};
use tower::ServiceExt;
use url::Url;
use uuid::Uuid;

struct TestIdentity {
    supabase_user_id: String,
    access_token: String,
}

fn env_value(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

async fn response_json(response: axum::response::Response) -> (StatusCode, Value) {
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1_000_000)
        .await
        .expect("response body");
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap_or(Value::Null)
    };
    (status, value)
}

async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    tenant: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
    }
    if let Some(tenant) = tenant {
        builder = builder.header("x-tenant-id", tenant);
    }
    let request = match body {
        Some(value) => builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(value.to_string()))
            .expect("request"),
        None => builder.body(Body::empty()).expect("request"),
    };
    response_json(app.clone().oneshot(request).await.expect("router")).await
}

async fn create_supabase_user(
    client: &reqwest::Client,
    base_url: &str,
    service_role: &str,
    email: &str,
    password: &str,
) -> String {
    let response = client
        .post(format!("{base_url}/auth/v1/admin/users"))
        .header("apikey", service_role)
        .header(header::AUTHORIZATION, format!("Bearer {service_role}"))
        .json(&json!({ "email": email, "password": password, "email_confirm": true }))
        .send()
        .await
        .expect("admin create user request");
    assert!(
        response.status().is_success(),
        "admin user creation failed with {}",
        response.status()
    );
    let body: Value = response.json().await.expect("admin user body");
    body["id"].as_str().expect("admin user id").to_owned()
}

async fn sign_in(
    client: &reqwest::Client,
    base_url: &str,
    anon_key: &str,
    email: &str,
    password: &str,
) -> String {
    let response = client
        .post(format!("{base_url}/auth/v1/token?grant_type=password"))
        .header("apikey", anon_key)
        .json(&json!({ "email": email, "password": password }))
        .send()
        .await
        .expect("password grant request");
    assert!(
        response.status().is_success(),
        "password sign-in failed with {}",
        response.status()
    );
    let body: Value = response.json().await.expect("token body");
    body["access_token"]
        .as_str()
        .expect("access token")
        .to_owned()
}

async fn delete_supabase_user(
    client: &reqwest::Client,
    base_url: &str,
    service_role: &str,
    user_id: &str,
) {
    let _ = client
        .delete(format!("{base_url}/auth/v1/admin/users/{user_id}"))
        .header("apikey", service_role)
        .header(header::AUTHORIZATION, format!("Bearer {service_role}"))
        .send()
        .await;
}

async fn cleanup(pool: &PgPool, tenant_ids: &[Uuid], user_ids: &[Uuid]) {
    // Deleting the tenant cascades to memberships, collects, submissions and audit rows.
    let _ = sqlx::query("DELETE FROM school_collect.tenants WHERE id = ANY($1)")
        .bind(tenant_ids)
        .execute(pool)
        .await;
    let _ = sqlx::query("DELETE FROM school_collect.users WHERE id = ANY($1)")
        .bind(user_ids)
        .execute(pool)
        .await;
}

#[tokio::test]
async fn authenticated_collect_flow_end_to_end() {
    let (Some(database_url), Some(supabase_url), Some(anon_key), Some(service_role)) = (
        env_value("DATABASE_URL"),
        env_value("SUPABASE_URL"),
        env_value("SUPABASE_ANON_KEY"),
        env_value("SUPABASE_SERVICE_ROLE_KEY"),
    ) else {
        eprintln!("collect_flow_e2e skipped: integration environment variables are not set");
        return;
    };

    let issuer = Url::parse(&format!("{supabase_url}/auth/v1")).expect("issuer url");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .after_connect(|connection, _meta| {
            Box::pin(async move {
                sqlx::query("SET search_path TO school_collect,public")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&database_url)
        .await
        .expect("connect");
    school_collect_db::MIGRATOR
        .run(&pool)
        .await
        .expect("migrate");

    let client = reqwest::Client::builder().build().expect("http client");
    let run_id = Uuid::now_v7().simple().to_string();

    let mut identities: Vec<TestIdentity> = Vec::new();
    let mut tenant_ids: Vec<Uuid> = Vec::new();
    let mut local_user_ids: Vec<Uuid> = Vec::new();

    // Sign in two unrelated schools so tenant isolation is a real assertion.
    for school in ["e2e-alpha", "e2e-beta"] {
        let email = format!("{school}-{run_id}@example.test");
        let password = format!("It3st-{run_id}!");
        let supabase_user_id =
            create_supabase_user(&client, &supabase_url, &service_role, &email, &password).await;
        let access_token = sign_in(&client, &supabase_url, &anon_key, &email, &password).await;
        identities.push(TestIdentity {
            supabase_user_id,
            access_token,
        });
    }

    // Fail with the provider's own reason before exercising the HTTP surface.
    let verifier = OidcVerifier::new(OidcConfig {
        issuer_url: issuer.clone(),
        audience: "authenticated".to_owned(),
        jwks_url: None,
    })
    .expect("verifier");
    for identity in &identities {
        if let Err(error) = verifier
            .verify(&format!("Bearer {}", identity.access_token))
            .await
        {
            for identity in &identities {
                delete_supabase_user(
                    &client,
                    &supabase_url,
                    &service_role,
                    &identity.supabase_user_id,
                )
                .await;
            }
            panic!("the configured verifier rejected a freshly issued access token: {error}");
        }
    }

    let app = router(
        AppState { pool: pool.clone() },
        HeaderValue::from_static("http://127.0.0.1:1420"),
        AuthState::oidc(verifier),
    );

    let result: Result<(), String> = async {
        let alpha = &identities[0];
        let beta = &identities[1];

        // The verified token is exchanged for a local user row.
        let (status, session) = call(
            &app,
            "GET",
            "/v1/session",
            Some(&alpha.access_token),
            None,
            None,
        )
        .await;
        if status != StatusCode::OK {
            return Err(format!("session failed: {status} {session}"));
        }
        let alpha_user_id = Uuid::parse_str(session["user"]["id"].as_str().unwrap_or_default())
            .map_err(|_| "session did not return a user id".to_owned())?;
        local_user_ids.push(alpha_user_id);
        if session["memberships"].as_array().map(Vec::is_empty) != Some(true) {
            return Err("a brand new identity must not already have memberships".to_owned());
        }

        let (status, tenant) = call(
            &app,
            "POST",
            "/v1/tenants",
            Some(&alpha.access_token),
            None,
            Some(json!({ "name": "E2E Alpha School" })),
        )
        .await;
        if status != StatusCode::CREATED {
            return Err(format!("tenant creation failed: {status} {tenant}"));
        }
        let alpha_tenant = tenant["tenantId"].as_str().unwrap_or_default().to_owned();
        let alpha_tenant_id = Uuid::parse_str(&alpha_tenant).map_err(|_| "tenant id".to_owned())?;
        tenant_ids.push(alpha_tenant_id);

        let (status, beta_session) = call(
            &app,
            "GET",
            "/v1/session",
            Some(&beta.access_token),
            None,
            None,
        )
        .await;
        if status != StatusCode::OK {
            return Err(format!("beta session failed: {status} {beta_session}"));
        }
        let beta_user_id = Uuid::parse_str(beta_session["user"]["id"].as_str().unwrap_or_default())
            .map_err(|_| "beta user id".to_owned())?;
        local_user_ids.push(beta_user_id);

        // A second school cannot see the first school's data.
        let (status, _) = call(
            &app,
            "GET",
            "/v1/collects",
            Some(&beta.access_token),
            Some(&alpha_tenant),
            None,
        )
        .await;
        if status != StatusCode::FORBIDDEN {
            return Err(format!("cross-tenant read was not refused: {status}"));
        }

        let (status, draft) = call(
            &app,
            "POST",
            "/v1/collects",
            Some(&alpha.access_token),
            Some(&alpha_tenant),
            Some(json!({
                "title": "E2E 수합",
                "description": "e2e",
                "items": [
                    { "key": "plan", "label": "계획서", "required": true },
                    { "key": "budget", "label": "예산안", "required": false }
                ]
            })),
        )
        .await;
        if status != StatusCode::CREATED {
            return Err(format!("collect creation failed: {status} {draft}"));
        }
        let collect_id = draft["id"].as_str().unwrap_or_default().to_owned();
        if draft["status"] != "draft" {
            return Err("a new collect must start as draft".to_owned());
        }

        // The draft carries the item definitions the submissions must answer,
        // and the default target list covers the school's members.
        let (status, detail) = call(
            &app,
            "GET",
            &format!("/v1/collects/{collect_id}"),
            Some(&alpha.access_token),
            Some(&alpha_tenant),
            None,
        )
        .await;
        if status != StatusCode::OK {
            return Err(format!("collect detail failed: {status} {detail}"));
        }
        if detail["items"].as_array().map(Vec::len) != Some(2) {
            return Err(format!("collect items were not stored: {detail}"));
        }
        if detail["items"][0]["key"] != "plan" || detail["items"][0]["required"] != true {
            return Err(format!("item definition came back wrong: {detail}"));
        }
        if detail["progress"]["assigned"].as_i64().unwrap_or_default() < 1 {
            return Err(format!("the target list was not created: {detail}"));
        }

        // Item keys become submission payload keys, so they are validated.
        let (status, invalid) = call(
            &app,
            "POST",
            "/v1/collects",
            Some(&alpha.access_token),
            Some(&alpha_tenant),
            Some(json!({
                "title": "invalid items",
                "items": [{ "key": "Not Valid", "label": "x" }]
            })),
        )
        .await;
        if status != StatusCode::BAD_REQUEST || invalid["code"] != "invalid_item_key" {
            return Err(format!(
                "an invalid item key was accepted: {status} {invalid}"
            ));
        }

        // Drafts cannot be edited before publication.
        let (status, _) = call(
            &app,
            "PUT",
            &format!("/v1/collects/{collect_id}/submission"),
            Some(&alpha.access_token),
            Some(&alpha_tenant),
            Some(json!({ "expectedVersion": 0, "payload": { "note": "too early" } })),
        )
        .await;
        if status != StatusCode::CONFLICT {
            return Err(format!(
                "editing an unpublished collect was not refused: {status}"
            ));
        }

        let (status, published) = call(
            &app,
            "POST",
            &format!("/v1/collects/{collect_id}/publish"),
            Some(&alpha.access_token),
            Some(&alpha_tenant),
            None,
        )
        .await;
        if status != StatusCode::OK || published["status"] != "published" {
            return Err(format!("publish failed: {status} {published}"));
        }

        // The manager view answers "who still owes this collect".
        let (status, progress) = call(
            &app,
            "GET",
            &format!("/v1/collects/{collect_id}/status"),
            Some(&alpha.access_token),
            Some(&alpha_tenant),
            None,
        )
        .await;
        if status != StatusCode::OK
            || progress["assigned"].as_i64().unwrap_or_default() < 1
            || progress["submitted"].as_i64().unwrap_or_default() != 0
        {
            return Err(format!(
                "status after publish was wrong: {status} {progress}"
            ));
        }
        if progress["rows"][0]["assignmentStatus"] != "assigned" {
            return Err(format!("assignment state was wrong: {progress}"));
        }

        let (status, assignments) = call(
            &app,
            "GET",
            "/v1/assignments",
            Some(&alpha.access_token),
            Some(&alpha_tenant),
            None,
        )
        .await;
        if status != StatusCode::OK
            || assignments["assignments"].as_array().map(Vec::len) != Some(1)
        {
            return Err(format!("my assignments were wrong: {status} {assignments}"));
        }
        if assignments["assignments"][0]["collectId"] != collect_id.as_str() {
            return Err(format!(
                "assignment did not reference the collect: {assignments}"
            ));
        }

        let (status, saved) = call(
            &app,
            "PUT",
            &format!("/v1/collects/{collect_id}/submission"),
            Some(&alpha.access_token),
            Some(&alpha_tenant),
            Some(json!({ "expectedVersion": 0, "payload": { "plan": "first" } })),
        )
        .await;
        if status != StatusCode::OK || saved["version"] != 1 {
            return Err(format!("first draft save failed: {status} {saved}"));
        }

        // A stale version must be reported, not silently overwritten.
        let (status, conflict) = call(
            &app,
            "PUT",
            &format!("/v1/collects/{collect_id}/submission"),
            Some(&alpha.access_token),
            Some(&alpha_tenant),
            Some(json!({ "expectedVersion": 0, "payload": { "plan": "stale" } })),
        )
        .await;
        if status != StatusCode::CONFLICT
            || conflict["code"] != "version_conflict"
            || conflict["currentVersion"] != 1
        {
            return Err(format!("stale write was not reported: {status} {conflict}"));
        }

        let (status, submitted) = call(
            &app,
            "POST",
            &format!("/v1/collects/{collect_id}/submission/submit"),
            Some(&alpha.access_token),
            Some(&alpha_tenant),
            None,
        )
        .await;
        if status != StatusCode::OK || submitted["status"] != "submitted" {
            return Err(format!("submit failed: {status} {submitted}"));
        }

        let (status, progress) = call(
            &app,
            "GET",
            &format!("/v1/collects/{collect_id}/status"),
            Some(&alpha.access_token),
            Some(&alpha_tenant),
            None,
        )
        .await;
        if status != StatusCode::OK || progress["submitted"].as_i64().unwrap_or_default() != 1 {
            return Err(format!(
                "the submitted count did not move: {status} {progress}"
            ));
        }
        if progress["rows"][0]["submissionStatus"] != "submitted" {
            return Err(format!("submission state was wrong: {progress}"));
        }

        let (status, _) = call(
            &app,
            "PUT",
            &format!("/v1/collects/{collect_id}/submission"),
            Some(&alpha.access_token),
            Some(&alpha_tenant),
            Some(json!({ "expectedVersion": 2, "payload": { "plan": "after submit" } })),
        )
        .await;
        if status != StatusCode::CONFLICT {
            return Err(format!(
                "editing a sent submission was not refused: {status}"
            ));
        }

        let (status, closed) = call(
            &app,
            "POST",
            &format!("/v1/collects/{collect_id}/close"),
            Some(&alpha.access_token),
            Some(&alpha_tenant),
            None,
        )
        .await;
        if status != StatusCode::OK || closed["status"] != "closed" {
            return Err(format!("close failed: {status} {closed}"));
        }

        // A closed collect is read-only.
        let (status, _) = call(
            &app,
            "PUT",
            &format!("/v1/collects/{collect_id}/submission"),
            Some(&alpha.access_token),
            Some(&alpha_tenant),
            Some(json!({ "expectedVersion": 2, "payload": { "plan": "after close" } })),
        )
        .await;
        if status != StatusCode::CONFLICT {
            return Err(format!(
                "editing a closed collect was not refused: {status}"
            ));
        }

        let (status, detail) = call(
            &app,
            "GET",
            &format!("/v1/collects/{collect_id}"),
            Some(&alpha.access_token),
            Some(&alpha_tenant),
            None,
        )
        .await;
        if status != StatusCode::OK
            || detail["submission"]["status"] != "submitted"
            || detail["submission"]["payload"]["plan"] != "first"
        {
            return Err(format!(
                "stored submission was not readable: {status} {detail}"
            ));
        }

        // Results stay exportable after the collect closes, and the file keeps
        // the member row even though the export runs in a later state.
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/v1/collects/{collect_id}/export"))
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {}", alpha.access_token),
                    )
                    .header("x-tenant-id", &alpha_tenant)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router");
        if response.status() != StatusCode::OK {
            return Err(format!("export after close failed: {}", response.status()));
        }
        let bytes = axum::body::to_bytes(response.into_body(), 1_000_000)
            .await
            .expect("csv body");
        let csv = String::from_utf8(bytes.to_vec()).expect("utf8 csv");
        if !csv.contains("first") {
            return Err(format!("exported csv lost the answer: {csv}"));
        }

        // The other school's admin cannot export this collect even with a valid
        // token: the tenant boundary is checked before anything is read.
        let (status, refused) = call(
            &app,
            "GET",
            &format!("/v1/collects/{collect_id}/export"),
            Some(&beta.access_token),
            Some(&alpha_tenant),
            None,
        )
        .await;
        if status != StatusCode::FORBIDDEN {
            return Err(format!(
                "cross-school export was not refused: {status} {refused}"
            ));
        }

        Ok(())
    }
    .await;

    // Always remove what the test created, including identity-provider records.
    for identity in &identities {
        delete_supabase_user(
            &client,
            &supabase_url,
            &service_role,
            &identity.supabase_user_id,
        )
        .await;
    }
    cleanup(&pool, &tenant_ids, &local_user_ids).await;

    if let Err(message) = result {
        panic!("collect flow end-to-end verification failed: {message}");
    }
}

/// Verifies the invitation path a teacher takes to join a school: the admin
/// issues a code, only the invited address can use it, and the new membership
/// can be assigned and submit. It runs against the real provider and database,
/// and skips silently when the integration environment is not configured.
/// Result export: an authorized member gets every assigned row, an unauthorized
/// member is refused, and a missing answer stays visible in the file.
#[tokio::test]
async fn result_export_end_to_end() {
    let (Some(database_url), Some(supabase_url), Some(anon_key), Some(service_role)) = (
        env_value("DATABASE_URL"),
        env_value("SUPABASE_URL"),
        env_value("SUPABASE_ANON_KEY"),
        env_value("SUPABASE_SERVICE_ROLE_KEY"),
    ) else {
        eprintln!("result_export_e2e skipped: integration environment variables are not set");
        return;
    };

    let issuer = Url::parse(&format!("{supabase_url}/auth/v1")).expect("issuer url");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .after_connect(|connection, _meta| {
            Box::pin(async move {
                sqlx::query("SET search_path TO school_collect,public")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&database_url)
        .await
        .expect("connect");
    school_collect_db::MIGRATOR
        .run(&pool)
        .await
        .expect("migrate");

    let client = reqwest::Client::builder().build().expect("http client");
    let run_id = Uuid::now_v7().simple().to_string();
    let coordinator_email = format!("export-coordinator-{run_id}@example.test");
    let contributor_email = format!("export-contributor-{run_id}@example.test");
    let password = format!("It3st-{run_id}!");

    let coordinator_supabase = create_supabase_user(
        &client,
        &supabase_url,
        &service_role,
        &coordinator_email,
        &password,
    )
    .await;
    let contributor_supabase = create_supabase_user(
        &client,
        &supabase_url,
        &service_role,
        &contributor_email,
        &password,
    )
    .await;
    let coordinator = sign_in(
        &client,
        &supabase_url,
        &anon_key,
        &coordinator_email,
        &password,
    )
    .await;
    let contributor = sign_in(
        &client,
        &supabase_url,
        &anon_key,
        &contributor_email,
        &password,
    )
    .await;

    let app = router(
        AppState { pool: pool.clone() },
        HeaderValue::from_static("http://127.0.0.1:1420"),
        AuthState::oidc(
            OidcVerifier::new(OidcConfig {
                issuer_url: issuer.clone(),
                audience: "authenticated".to_owned(),
                jwks_url: None,
            })
            .expect("verifier"),
        ),
    );

    let mut tenant_ids: Vec<Uuid> = Vec::new();
    let mut local_user_ids: Vec<Uuid> = Vec::new();
    let result = async {
        let (status, tenant) = call(
            &app,
            "POST",
            "/v1/tenants",
            Some(&coordinator),
            None,
            Some(json!({ "name": format!("export-{run_id}") })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "tenant create: {tenant}");
        let tenant_id = tenant["tenantId"].as_str().expect("tenant id").to_owned();
        tenant_ids.push(Uuid::parse_str(&tenant_id).expect("tenant uuid"));

        // The contributor joins through an invitation, exactly as a teacher would.
        let (status, invitation) = call(
            &app,
            "POST",
            "/v1/invitations",
            Some(&coordinator),
            Some(&tenant_id),
            Some(json!({ "email": contributor_email })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "invitation: {invitation}");
        let code = invitation["code"].as_str().expect("invite code").to_owned();
        let (status, accepted) = call(
            &app,
            "POST",
            "/v1/invitations/accept",
            Some(&contributor),
            None,
            Some(json!({ "code": code })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "accept: {accepted}");

        let (status, collect) = call(
            &app,
            "POST",
            "/v1/collects",
            Some(&coordinator),
            Some(&tenant_id),
            Some(json!({
                "title": format!("export-{run_id}"),
                "description": "결과 내보내기 검증",
                "items": [{ "key": "note", "label": "내용", "required": true }],
                "assigneeUserIds": [],
            })),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED, "collect: {collect}");
        let collect_id = collect["id"].as_str().expect("collect id").to_owned();

        let (status, published) = call(
            &app,
            "POST",
            &format!("/v1/collects/{collect_id}/publish"),
            Some(&coordinator),
            Some(&tenant_id),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "publish: {published}");

        // Only the contributor answers; the coordinator stays unsubmitted.
        let (status, saved) = call(
            &app,
            "PUT",
            &format!("/v1/collects/{collect_id}/submission"),
            Some(&contributor),
            Some(&tenant_id),
            Some(json!({ "expectedVersion": 0, "payload": { "note": "값, 쉼표 포함" } })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "save: {saved}");
        let (status, submitted) = call(
            &app,
            "POST",
            &format!("/v1/collects/{collect_id}/submission/submit"),
            Some(&contributor),
            Some(&tenant_id),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "submit: {submitted}");

        // The coordinator exports: every assigned member appears.
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/v1/collects/{collect_id}/export"))
                    .header(header::AUTHORIZATION, format!("Bearer {coordinator}"))
                    .header("x-tenant-id", &tenant_id)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("router");
        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            response
                .headers()
                .get(header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .starts_with("text/csv")
        );
        let bytes = axum::body::to_bytes(response.into_body(), 1_000_000)
            .await
            .expect("csv body");
        let csv = String::from_utf8(bytes.to_vec()).expect("utf8 csv");
        let contributor_name = contributor_email.split('@').next().expect("name");
        assert!(csv.contains("내용"), "header carries the item label");
        assert!(
            csv.contains(contributor_name),
            "the contributor row is present"
        );
        assert!(
            csv.contains("\"값, 쉼표 포함\""),
            "values stay quoted: {csv}"
        );
        assert_eq!(
            csv.lines().count(),
            3,
            "header plus both assigned members: {csv}"
        );

        // A contributor cannot export other people's answers.
        let (status, refused) = call(
            &app,
            "GET",
            &format!("/v1/collects/{collect_id}/export"),
            Some(&contributor),
            Some(&tenant_id),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::FORBIDDEN, "refused: {refused}");

        // The export is recorded for the audit trail.
        let recorded = sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM school_collect.audit_events
             WHERE tenant_id = $1 AND action = 'collect.exported'",
        )
        .bind(tenant_ids[0])
        .fetch_one(&pool)
        .await
        .expect("audit count");
        assert_eq!(recorded, 1, "one export is recorded");

        local_user_ids = sqlx::query_scalar::<_, Uuid>(
            "SELECT id FROM school_collect.users WHERE subject = ANY($1)",
        )
        .bind(vec![
            coordinator_supabase.clone(),
            contributor_supabase.clone(),
        ])
        .fetch_all(&pool)
        .await
        .expect("local users");
    }
    .await;

    cleanup(&pool, &tenant_ids, &local_user_ids).await;
    delete_supabase_user(&client, &supabase_url, &service_role, &coordinator_supabase).await;
    delete_supabase_user(&client, &supabase_url, &service_role, &contributor_supabase).await;
    result
}

#[tokio::test]
async fn membership_invitation_end_to_end() {
    let (Some(database_url), Some(supabase_url), Some(anon_key), Some(service_role)) = (
        env_value("DATABASE_URL"),
        env_value("SUPABASE_URL"),
        env_value("SUPABASE_ANON_KEY"),
        env_value("SUPABASE_SERVICE_ROLE_KEY"),
    ) else {
        eprintln!(
            "membership_invitation_e2e skipped: integration environment variables are not set"
        );
        return;
    };

    let issuer = Url::parse(&format!("{supabase_url}/auth/v1")).expect("issuer url");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .after_connect(|connection, _meta| {
            Box::pin(async move {
                sqlx::query("SET search_path TO school_collect,public")
                    .execute(connection)
                    .await?;
                Ok(())
            })
        })
        .connect(&database_url)
        .await
        .expect("connect");
    school_collect_db::MIGRATOR
        .run(&pool)
        .await
        .expect("migrate");

    let client = reqwest::Client::builder().build().expect("http client");
    let run_id = Uuid::now_v7().simple().to_string();

    // admin: issues invitations. teacher: the invited address. other: a third
    // account that must not be able to use someone else's code. expired: an
    // invited address whose invitation lapses before acceptance.
    let mut identities: Vec<(String, TestIdentity)> = Vec::new();
    for kind in ["admin", "teacher", "other", "expired"] {
        let email = format!("invite-{kind}-{run_id}@example.test");
        let password = format!("It3st-{run_id}!");
        let supabase_user_id =
            create_supabase_user(&client, &supabase_url, &service_role, &email, &password).await;
        let access_token = sign_in(&client, &supabase_url, &anon_key, &email, &password).await;
        identities.push((
            email,
            TestIdentity {
                supabase_user_id,
                access_token,
            },
        ));
    }

    let verifier = OidcVerifier::new(OidcConfig {
        issuer_url: issuer.clone(),
        audience: "authenticated".to_owned(),
        jwks_url: None,
    })
    .expect("verifier");
    let app = router(
        AppState { pool: pool.clone() },
        HeaderValue::from_static("http://127.0.0.1:1420"),
        AuthState::oidc(verifier),
    );

    let mut tenant_ids: Vec<Uuid> = Vec::new();
    let mut local_user_ids: Vec<Uuid> = Vec::new();

    let result: Result<(), String> = async {
        let admin = &identities[0].1;
        let teacher = &identities[1].1;
        let other = &identities[2].1;
        let expired_invitee = &identities[3].1;
        let teacher_email = identities[1].0.as_str();
        let expired_email = identities[3].0.as_str();

        // Every identity is exchanged for a local user row on first use.
        for identity in [admin, teacher, other, expired_invitee] {
            let (status, session) = call(
                &app,
                "GET",
                "/v1/session",
                Some(&identity.access_token),
                None,
                None,
            )
            .await;
            if status != StatusCode::OK {
                return Err(format!("session failed: {status} {session}"));
            }
            let user_id = Uuid::parse_str(session["user"]["id"].as_str().unwrap_or_default())
                .map_err(|_| "session did not return a user id".to_owned())?;
            local_user_ids.push(user_id);
        }

        let (status, tenant) = call(
            &app,
            "POST",
            "/v1/tenants",
            Some(&admin.access_token),
            None,
            Some(json!({ "name": "E2E Invitation School" })),
        )
        .await;
        if status != StatusCode::CREATED {
            return Err(format!("tenant creation failed: {status} {tenant}"));
        }
        let tenant_id = tenant["tenantId"].as_str().unwrap_or_default().to_owned();
        tenant_ids.push(Uuid::parse_str(&tenant_id).map_err(|_| "tenant id".to_owned())?);

        // The manager invites the teacher; the code is returned exactly once.
        let (status, created) = call(
            &app,
            "POST",
            "/v1/invitations",
            Some(&admin.access_token),
            Some(&tenant_id),
            Some(json!({ "email": teacher_email })),
        )
        .await;
        if status != StatusCode::CREATED {
            return Err(format!("invitation creation failed: {status} {created}"));
        }
        if created["invitation"]["role"] != "contributor"
            || created["invitation"]["status"] != "pending"
        {
            return Err(format!(
                "the default invitation role or status is wrong: {created}"
            ));
        }
        let code = created["code"].as_str().unwrap_or_default().to_owned();
        let invitation_id = created["invitation"]["id"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        if code.len() != 64 || !code.chars().all(|value| value.is_ascii_hexdigit()) {
            return Err(format!("the invite code shape is wrong: {code}"));
        }

        // The stored value must be a hash, never the code itself.
        let stored_hash: Option<String> =
            sqlx::query_scalar("SELECT code_hash FROM school_collect.invitations WHERE id = $1")
                .bind(Uuid::parse_str(&invitation_id).map_err(|_| "invitation id".to_owned())?)
                .fetch_optional(&pool)
                .await
                .map_err(|error| format!("code hash lookup failed: {error}"))?;
        match stored_hash {
            Some(hash) if hash.len() == 64 && hash != code => {}
            other => {
                return Err(format!(
                    "the invitation code must be stored hashed: {other:?}"
                ));
            }
        }

        // One pending invitation per address and school.
        let (status, duplicate) = call(
            &app,
            "POST",
            "/v1/invitations",
            Some(&admin.access_token),
            Some(&tenant_id),
            Some(json!({ "email": teacher_email })),
        )
        .await;
        if status != StatusCode::CONFLICT || duplicate["code"] != "invitation_exists" {
            return Err(format!(
                "a duplicate pending invitation was not refused: {status} {duplicate}"
            ));
        }

        // An invitation can never hand out admin rights.
        let (status, refused) = call(
            &app,
            "POST",
            "/v1/invitations",
            Some(&admin.access_token),
            Some(&tenant_id),
            Some(json!({ "email": "admin-target@example.test", "role": "admin" })),
        )
        .await;
        if status != StatusCode::BAD_REQUEST || refused["code"] != "invalid_role" {
            return Err(format!(
                "an admin invitation was not refused: {status} {refused}"
            ));
        }

        // Someone else's address cannot use the code.
        let (status, mismatch) = call(
            &app,
            "POST",
            "/v1/invitations/accept",
            Some(&other.access_token),
            None,
            Some(json!({ "code": code })),
        )
        .await;
        if status != StatusCode::FORBIDDEN || mismatch["code"] != "invitation_email_mismatch" {
            return Err(format!(
                "another address accepted the invitation: {status} {mismatch}"
            ));
        }

        // The invited address joins and receives the stored role.
        let (status, accepted) = call(
            &app,
            "POST",
            "/v1/invitations/accept",
            Some(&teacher.access_token),
            None,
            Some(json!({ "code": code })),
        )
        .await;
        if status != StatusCode::OK {
            return Err(format!("accept failed: {status} {accepted}"));
        }
        if accepted["membership"]["role"] != "contributor"
            || accepted["membership"]["tenantId"] != tenant_id
        {
            return Err(format!(
                "the membership after acceptance is wrong: {accepted}"
            ));
        }

        // The code is single use.
        let (status, reused) = call(
            &app,
            "POST",
            "/v1/invitations/accept",
            Some(&teacher.access_token),
            None,
            Some(json!({ "code": code })),
        )
        .await;
        if status != StatusCode::CONFLICT || reused["code"] != "invitation_used" {
            return Err(format!("the code was accepted twice: {status} {reused}"));
        }

        // The new member can be assigned, write a draft, and submit.
        let (status, draft) = call(
            &app,
            "POST",
            "/v1/collects",
            Some(&admin.access_token),
            Some(&tenant_id),
            Some(json!({
                "title": "E2E 초대 수합",
                "items": [{ "key": "plan", "label": "계획서", "required": true }]
            })),
        )
        .await;
        if status != StatusCode::CREATED {
            return Err(format!("collect creation failed: {status} {draft}"));
        }
        let collect_id = draft["id"].as_str().unwrap_or_default().to_owned();

        let (status, published) = call(
            &app,
            "POST",
            &format!("/v1/collects/{collect_id}/publish"),
            Some(&admin.access_token),
            Some(&tenant_id),
            None,
        )
        .await;
        if status != StatusCode::OK {
            return Err(format!("publish failed: {status} {published}"));
        }

        let (status, assignments) = call(
            &app,
            "GET",
            "/v1/assignments",
            Some(&teacher.access_token),
            Some(&tenant_id),
            None,
        )
        .await;
        if status != StatusCode::OK {
            return Err(format!("assignments failed: {status} {assignments}"));
        }
        let assigned = assignments["assignments"]
            .as_array()
            .map(|rows| {
                rows.iter()
                    .any(|row| row["collectId"] == collect_id.as_str())
            })
            .unwrap_or(false);
        if !assigned {
            return Err(format!(
                "the newly joined member was not assigned: {assignments}"
            ));
        }

        let (status, saved) = call(
            &app,
            "PUT",
            &format!("/v1/collects/{collect_id}/submission"),
            Some(&teacher.access_token),
            Some(&tenant_id),
            Some(json!({ "expectedVersion": 0, "payload": { "plan": "제출" } })),
        )
        .await;
        if status != StatusCode::OK {
            return Err(format!("draft save failed: {status} {saved}"));
        }
        let (status, submitted) = call(
            &app,
            "POST",
            &format!("/v1/collects/{collect_id}/submission/submit"),
            Some(&teacher.access_token),
            Some(&tenant_id),
            None,
        )
        .await;
        if status != StatusCode::OK || submitted["status"] != "submitted" {
            return Err(format!("submit failed: {status} {submitted}"));
        }

        let (status, progress) = call(
            &app,
            "GET",
            &format!("/v1/collects/{collect_id}/status"),
            Some(&admin.access_token),
            Some(&tenant_id),
            None,
        )
        .await;
        if status != StatusCode::OK || progress["submitted"].as_i64() != Some(1) {
            return Err(format!(
                "the manager view did not count the new submission: {status} {progress}"
            ));
        }

        // Revoking only applies to a pending invitation.
        let second_email = format!("revoked-{run_id}@example.test");
        let (status, second) = call(
            &app,
            "POST",
            "/v1/invitations",
            Some(&admin.access_token),
            Some(&tenant_id),
            Some(json!({ "email": second_email, "role": "viewer" })),
        )
        .await;
        if status != StatusCode::CREATED {
            return Err(format!("second invitation failed: {status} {second}"));
        }
        let second_id = second["invitation"]["id"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let (status, revoked) = call(
            &app,
            "POST",
            &format!("/v1/invitations/{second_id}/revoke"),
            Some(&admin.access_token),
            Some(&tenant_id),
            None,
        )
        .await;
        if status != StatusCode::OK || revoked["status"] != "revoked" {
            return Err(format!("revoke failed: {status} {revoked}"));
        }
        let (status, again) = call(
            &app,
            "POST",
            &format!("/v1/invitations/{second_id}/revoke"),
            Some(&admin.access_token),
            Some(&tenant_id),
            None,
        )
        .await;
        if status != StatusCode::NOT_FOUND || again["code"] != "invitation_not_found" {
            return Err(format!(
                "revoking an already revoked invitation was not a 404: {status} {again}"
            ));
        }

        // An invitation that lapsed cannot be accepted.
        let (status, expired_created) = call(
            &app,
            "POST",
            "/v1/invitations",
            Some(&admin.access_token),
            Some(&tenant_id),
            Some(json!({ "email": expired_email })),
        )
        .await;
        if status != StatusCode::CREATED {
            return Err(format!(
                "expired invitation setup failed: {status} {expired_created}"
            ));
        }
        let expired_id = expired_created["invitation"]["id"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let expired_code = expired_created["code"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        sqlx::query(
            "UPDATE school_collect.invitations
                SET created_at = now() - interval '15 days',
                    expires_at = now() - interval '1 day'
              WHERE id = $1",
        )
        .bind(Uuid::parse_str(&expired_id).map_err(|_| "expired invitation id".to_owned())?)
        .execute(&pool)
        .await
        .map_err(|error| format!("expiry update failed: {error}"))?;
        let (status, expired) = call(
            &app,
            "POST",
            "/v1/invitations/accept",
            Some(&expired_invitee.access_token),
            None,
            Some(json!({ "code": expired_code })),
        )
        .await;
        if status != StatusCode::GONE || expired["code"] != "invitation_expired" {
            return Err(format!(
                "an expired invitation was accepted: {status} {expired}"
            ));
        }

        // The list keeps the accepted, revoked, and pending rows.
        let (status, list) = call(
            &app,
            "GET",
            "/v1/invitations",
            Some(&admin.access_token),
            Some(&tenant_id),
            None,
        )
        .await;
        if status != StatusCode::OK {
            return Err(format!("invitation list failed: {status} {list}"));
        }
        let rows = list["invitations"].as_array().cloned().unwrap_or_default();
        let has_state = |state: &str| rows.iter().any(|row| row["status"] == state);
        if !(has_state("accepted") && has_state("revoked") && has_state("pending")) {
            return Err(format!("the invitation list is missing states: {list}"));
        }

        // An account that never accepted the invitation cannot read the school.
        let (status, _) = call(
            &app,
            "GET",
            "/v1/collects",
            Some(&other.access_token),
            Some(&tenant_id),
            None,
        )
        .await;
        if status != StatusCode::FORBIDDEN {
            return Err(format!("an uninvited account read the school: {status}"));
        }

        Ok(())
    }
    .await;

    // Always remove what the test created, including identity-provider records.
    for (_, identity) in &identities {
        delete_supabase_user(
            &client,
            &supabase_url,
            &service_role,
            &identity.supabase_user_id,
        )
        .await;
    }
    cleanup(&pool, &tenant_ids, &local_user_ids).await;

    if let Err(message) = result {
        panic!("membership invitation end-to-end verification failed: {message}");
    }
}
