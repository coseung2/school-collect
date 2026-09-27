//! R2 adapter against a loopback S3 stand-in.
//!
//! The stand-in keeps objects in memory and refuses any request whose
//! `Authorization` header or presigned query does not carry the expected
//! access key and a SigV4 algorithm, so a wrong request shape fails here long
//! before a real bucket exists. The signature value itself is covered by the
//! AWS reference vectors in `r2.rs`.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Router,
    body::Bytes,
    extract::{Path, RawQuery, State},
    http::{HeaderMap, Method, StatusCode},
    response::IntoResponse,
    routing::any,
};
use chrono::Utc;
use school_collect_application::{
    r2::{R2Config, R2Storage},
    storage::ObjectStorage,
};

const ACCESS_KEY: &str = "test-access-key";

/// Object bytes and the content type they were stored with, by key.
type Objects = HashMap<String, (Vec<u8>, String)>;

#[derive(Clone, Default)]
struct Bucket {
    objects: Arc<Mutex<Objects>>,
    seen: Arc<Mutex<Vec<String>>>,
}

async fn handle(
    State(bucket): State<Bucket>,
    method: Method,
    Path((name, key)): Path<(String, String)>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let query = query.unwrap_or_default();
    let header_auth = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let signed_by_header = header_auth.starts_with("AWS4-HMAC-SHA256 ")
        && header_auth.contains(&format!("Credential={ACCESS_KEY}/"))
        && header_auth.contains("Signature=")
        && headers.contains_key("x-amz-date")
        && headers.contains_key("x-amz-content-sha256");
    let signed_by_query = query.contains("X-Amz-Algorithm=AWS4-HMAC-SHA256")
        && query.contains(&format!("X-Amz-Credential={ACCESS_KEY}%2F"))
        && query.contains("X-Amz-Signature=");
    if name != "school-collect-test" || !(signed_by_header || signed_by_query) {
        return (
            StatusCode::FORBIDDEN,
            "<Error><Code>AccessDenied</Code></Error>",
        )
            .into_response();
    }
    bucket.seen.lock().unwrap().push(format!(
        "{method} {key} {}",
        if signed_by_query {
            "presigned"
        } else {
            "header"
        }
    ));

    let mut objects = bucket.objects.lock().unwrap();
    match method {
        Method::PUT => {
            let content_type = headers
                .get("content-type")
                .and_then(|value| value.to_str().ok())
                .unwrap_or("application/octet-stream")
                .to_owned();
            objects.insert(key, (body.to_vec(), content_type));
            StatusCode::OK.into_response()
        }
        Method::GET => match objects.get(&key) {
            Some((bytes, _)) => (StatusCode::OK, bytes.clone()).into_response(),
            None => (
                StatusCode::NOT_FOUND,
                "<Error><Code>NoSuchKey</Code></Error>",
            )
                .into_response(),
        },
        Method::DELETE => {
            objects.remove(&key);
            StatusCode::NO_CONTENT.into_response()
        }
        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
    }
}

async fn start() -> (String, Bucket) {
    let bucket = Bucket::default();
    let app = Router::new()
        .route("/{name}/{*key}", any(handle))
        .with_state(bucket.clone());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (format!("http://{address}"), bucket)
}

fn config(endpoint: &str, secret: &str) -> R2Config {
    R2Config {
        endpoint: endpoint.to_owned(),
        bucket: "school-collect-test".into(),
        access_key_id: ACCESS_KEY.into(),
        secret_access_key: secret.into(),
        region: "auto".into(),
    }
}

#[tokio::test]
async fn the_adapter_round_trips_objects_through_an_s3_api() {
    let (endpoint, bucket) = start().await;
    let storage = R2Storage::new(config(&endpoint, "test-secret")).unwrap();
    let key = "tenants/0190-aa/attachments/0190-bb";

    assert_eq!(
        storage.get(key).await.unwrap(),
        None,
        "unknown keys are None"
    );
    storage
        .put(key, b"%PDF-1.4 test".to_vec(), "application/pdf")
        .await
        .unwrap();
    assert_eq!(
        bucket
            .objects
            .lock()
            .unwrap()
            .get(key)
            .map(|(_, t)| t.clone()),
        Some("application/pdf".into()),
        "the declared type reaches the bucket"
    );
    assert_eq!(
        storage.get(key).await.unwrap(),
        Some(b"%PDF-1.4 test".to_vec())
    );
    storage.delete(key).await.unwrap();
    assert_eq!(storage.get(key).await.unwrap(), None);
    storage.delete(key).await.unwrap(); // deleting twice is not an error

    // A presigned PUT lets a client upload without the API relaying bytes.
    let url = storage
        .presigned_url("PUT", key, Duration::from_secs(300), Utc::now())
        .unwrap();
    let response = reqwest::Client::new()
        .put(&url)
        .body("direct upload")
        .send()
        .await
        .unwrap();
    assert!(response.status().is_success(), "{}", response.status());
    assert_eq!(
        storage.get(key).await.unwrap(),
        Some(b"direct upload".to_vec())
    );

    let seen = bucket.seen.lock().unwrap().clone();
    assert!(
        seen.iter()
            .any(|line| line == &format!("PUT {key} presigned"))
    );
    assert!(seen.iter().all(|line| !line.contains("test-secret")));
}

#[tokio::test]
async fn provider_refusals_surface_as_storage_failures() {
    let (endpoint, _) = start().await;
    let mut wrong = config(&endpoint, "test-secret");
    wrong.access_key_id = "someone-else".into();
    let storage = R2Storage::new(wrong).unwrap();
    let error = storage
        .put("tenants/t/attachments/a", b"x".to_vec(), "text/plain")
        .await
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("403") && error.contains("AccessDenied"),
        "{error}"
    );
    assert!(
        !error.contains("test-secret"),
        "errors never echo the secret"
    );
}
