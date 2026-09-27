//! R2 adapter against a loopback S3 stand-in.
//!
//! The stand-in keeps objects in memory and recomputes the SigV4
//! `Authorization` header from what each request actually carries (method,
//! path, `Host`, `x-amz-date`, `x-amz-content-sha256` and content type),
//! refusing a mismatch with 403 `SignatureDoesNotMatch`. A signature that
//! covers a different host than the one on the wire is therefore caught here,
//! long before a real bucket exists. The signature value itself is also
//! checked against the AWS reference vectors in `r2.rs`.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use axum::{
    Router,
    body::Bytes,
    extract::{Path, RawQuery, State},
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::any,
};
use chrono::{DateTime, Utc};
use school_collect_application::{
    r2::{R2Config, R2Storage, authorization_header_for_test},
    storage::ObjectStorage,
};
use sha2::{Digest, Sha256};

const ACCESS_KEY: &str = "test-access-key";
const SECRET_KEY: &str = "test-secret";
const BUCKET: &str = "school-collect-test";

/// Object bytes and the content type they were stored with, by key.
type Objects = HashMap<String, (Vec<u8>, String)>;

#[derive(Clone)]
struct Stand {
    objects: Arc<Mutex<Objects>>,
    seen: Arc<Mutex<Vec<String>>>,
    config: Arc<R2Config>,
}

async fn handle(
    State(stand): State<Stand>,
    method: Method,
    uri: Uri,
    Path((name, key)): Path<(String, String)>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let query = query.unwrap_or_default();
    if name != BUCKET {
        return deny("AccessDenied");
    }
    let presigned = headers.get("authorization").is_none();
    if presigned {
        // The presigned round-trip only has to be accepted; its signature is
        // exercised by the AWS reference vector in `r2.rs`.
        if !query.contains("X-Amz-Algorithm=AWS4-HMAC-SHA256")
            || !query.contains(&format!("X-Amz-Credential={ACCESS_KEY}%2F"))
            || !query.contains("X-Amz-Signature=")
        {
            return deny("AccessDenied");
        }
    } else if let Err(code) =
        verify_header_signature(&stand.config, method.as_str(), uri.path(), &headers, &body)
    {
        return deny(code);
    }
    stand.seen.lock().unwrap().push(format!(
        "{method} {key} {}",
        if presigned { "presigned" } else { "header" }
    ));

    let mut objects = stand.objects.lock().unwrap();
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

fn deny(code: &str) -> Response {
    (
        StatusCode::FORBIDDEN,
        format!("<Error><Code>{code}</Code></Error>"),
    )
        .into_response()
}

/// Reads one `Key=value` field out of the comma-separated `Authorization`
/// header, stopping before the next field.
fn auth_field<'a>(auth: &'a str, key: &str) -> Option<&'a str> {
    let rest = auth.split_once(key)?.1;
    let end = rest.find(", ").unwrap_or(rest.len());
    Some(&rest[..end])
}

fn parse_amz_date(value: &str) -> Option<DateTime<Utc>> {
    chrono::NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%SZ")
        .ok()
        .map(|naive| naive.and_utc())
}

/// Recomputes the client's signature from the request as received.
///
/// The `Host` header, path, method, signed header set and payload hash all
/// come from the wire, so a signature computed over a different authority
/// (for example the raw, un-normalized endpoint) cannot match. Returns the S3
/// error code to answer with when the request is not acceptable.
fn verify_header_signature(
    config: &R2Config,
    method: &str,
    path: &str,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<(), &'static str> {
    let auth = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .ok_or("AccessDenied")?;
    if !auth.starts_with("AWS4-HMAC-SHA256 ") {
        return Err("AccessDenied");
    }
    let key_id = auth_field(auth, "Credential=")
        .and_then(|credential| credential.split_once('/'))
        .map(|(key_id, _scope)| key_id)
        .ok_or("AccessDenied")?;
    if key_id != config.access_key_id {
        return Err("AccessDenied");
    }

    let signed_headers = auth_field(auth, "SignedHeaders=").ok_or("SignatureDoesNotMatch")?;
    let mut signed = Vec::new();
    for name in signed_headers.split(';').filter(|name| !name.is_empty()) {
        let value = headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .ok_or("SignatureDoesNotMatch")?;
        signed.push((name.to_owned(), value.to_owned()));
    }

    let payload_hash = headers
        .get("x-amz-content-sha256")
        .and_then(|value| value.to_str().ok())
        .ok_or("SignatureDoesNotMatch")?;
    if payload_hash != hex::encode(Sha256::digest(body)) {
        return Err("XAmzContentSHA256Mismatch");
    }
    let date = headers
        .get("x-amz-date")
        .and_then(|value| value.to_str().ok())
        .and_then(parse_amz_date)
        .ok_or("SignatureDoesNotMatch")?;

    let expected = authorization_header_for_test(config, method, path, &signed, payload_hash, date);
    if expected != auth {
        return Err("SignatureDoesNotMatch");
    }
    Ok(())
}

async fn start() -> (R2Config, Stand) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let config = R2Config {
        endpoint: format!("http://{address}"),
        bucket: BUCKET.into(),
        access_key_id: ACCESS_KEY.into(),
        secret_access_key: SECRET_KEY.into(),
        region: "auto".into(),
    };
    let stand = Stand {
        objects: Arc::default(),
        seen: Arc::default(),
        config: Arc::new(config.clone()),
    };
    let app = Router::new()
        .route("/{name}/{*key}", any(handle))
        .with_state(stand.clone());
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (config, stand)
}

#[tokio::test]
async fn the_adapter_round_trips_objects_through_an_s3_api() {
    let (config, stand) = start().await;
    let storage = R2Storage::new(config).unwrap();
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
        stand
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

    let seen = stand.seen.lock().unwrap().clone();
    assert!(
        seen.iter()
            .any(|line| line == &format!("PUT {key} presigned"))
    );
    assert!(seen.iter().all(|line| !line.contains("test-secret")));
}

#[tokio::test]
async fn provider_refusals_surface_as_storage_failures() {
    let (mut wrong, _) = start().await;
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

/// Regression for the signing/transport host mismatch: the signature covers a
/// host without the port that the request actually carries.
#[tokio::test]
async fn a_request_signed_for_a_different_host_is_rejected() {
    let (config, _) = start().await;
    let key = "tenants/0190-aa/attachments/0190-bb";
    let path = format!("/{BUCKET}/{key}");
    let body = b"tampered";
    let payload_hash = hex::encode(Sha256::digest(body));
    let now = Utc::now();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();
    // Sign for the bare loopback host, then send to the endpoint that carries
    // its (non-default) port, so the signed `Host` differs from the wire one.
    let signed_host = config
        .endpoint
        .split_once("://")
        .unwrap()
        .1
        .split(':')
        .next()
        .unwrap()
        .to_owned();
    let signed = vec![
        ("host".to_owned(), signed_host),
        ("x-amz-content-sha256".to_owned(), payload_hash.clone()),
        ("x-amz-date".to_owned(), amz_date.clone()),
        ("content-type".to_owned(), "text/plain".to_owned()),
    ];
    let authorization =
        authorization_header_for_test(&config, "PUT", &path, &signed, &payload_hash, now);

    let response = reqwest::Client::new()
        .put(format!("{}{path}", config.endpoint))
        .header("authorization", authorization)
        .header("x-amz-content-sha256", payload_hash)
        .header("x-amz-date", amz_date)
        .header("content-type", "text/plain")
        .body(body.to_vec())
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let text = response.text().await.unwrap();
    assert!(text.contains("SignatureDoesNotMatch"), "{text}");
}
