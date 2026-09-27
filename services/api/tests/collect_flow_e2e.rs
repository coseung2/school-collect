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
            Some(json!({ "expectedVersion": 0, "payload": { "note": "first" } })),
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
            Some(json!({ "expectedVersion": 0, "payload": { "note": "stale" } })),
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
            Some(json!({ "expectedVersion": 2, "payload": { "note": "after submit" } })),
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
            Some(json!({ "expectedVersion": 2, "payload": { "note": "after close" } })),
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
            || detail["submission"]["payload"]["note"] != "first"
        {
            return Err(format!(
                "stored submission was not readable: {status} {detail}"
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
