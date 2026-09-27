//! Local bridge for the browser extension.
//!
//! The desktop app listens on `127.0.0.1` only. Every request must come from the
//! pinned extension origin and carry the pairing token that the app shows in the
//! 업무 자동화 screen. The bridge can only create the same shortcut recipes the
//! user can create by hand: it never receives cookies, tokens, or page content,
//! and it never opens arbitrary URLs.

use std::{fs, path::PathBuf, sync::Arc};

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tower_http::cors::{AllowOrigin, CorsLayer};
use uuid::Uuid;

use crate::{
    AutomationKind, AutomationRecipe, automation_config_dir, automation_recipes_path,
    normalize_target_url, upsert_automation_recipe, validate_recipe, write_file_atomically,
};

/// Loopback port the desktop app listens on for the extension.
pub const BRIDGE_PORT: u16 = 43110;
/// Pairing token file inside the app config directory.
pub const BRIDGE_TOKEN_FILE: &str = "automation-bridge.token";
/// Chrome extension id derived from the public key pinned in
/// `apps/extension/manifest.json`. Loading that folder as an unpacked extension
/// produces this id on every machine, so the allowlist stays a constant.
pub const ALLOWED_EXTENSION_ID: &str = "dfobjphjganlegjomdgmaaphbjcgpoea";

fn allowed_origin() -> String {
    format!("chrome-extension://{ALLOWED_EXTENSION_ID}")
}

#[derive(Clone)]
pub struct BridgeState {
    pub token: Arc<String>,
    pub recipes_path: PathBuf,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeStatus {
    pub service: &'static str,
    pub version: &'static str,
    pub port: u16,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutRequest {
    pub name: String,
    pub url: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutResponse {
    pub id: String,
    pub name: String,
    pub target_url: String,
}

fn json_error(status: StatusCode, code: &str, message: &str) -> Response {
    (
        status,
        Json(serde_json::json!({ "code": code, "message": message })),
    )
        .into_response()
}

/// Constant-time comparison so a wrong token cannot be guessed byte by byte.
fn constant_time_eq(left: &str, right: &str) -> bool {
    let left = left.as_bytes();
    let right = right.as_bytes();
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0u8;
    for (a, b) in left.iter().zip(right.iter()) {
        difference |= a ^ b;
    }
    difference == 0
}

pub fn router(state: BridgeState) -> Router {
    let origin = HeaderValue::from_str(&allowed_origin())
        .expect("the pinned extension origin is a valid header value");
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::exact(origin))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE]);

    Router::new()
        .route("/v1/bridge/status", get(status))
        .route("/v1/bridge/shortcuts", post(create_shortcut))
        .route_layer(middleware::from_fn_with_state(state.clone(), guard))
        .with_state(state)
        .layer(cors)
}

async fn status() -> Json<BridgeStatus> {
    Json(BridgeStatus {
        service: "school-collect-automation-bridge",
        version: env!("CARGO_PKG_VERSION"),
        port: BRIDGE_PORT,
    })
}

async fn create_shortcut(
    State(state): State<BridgeState>,
    Json(body): Json<ShortcutRequest>,
) -> Response {
    let name = body.name.trim();
    if name.is_empty() || name.chars().count() > 80 {
        return json_error(
            StatusCode::BAD_REQUEST,
            "invalid_name",
            "a button name between 1 and 80 characters is required",
        );
    }
    let Ok(target_url) = normalize_target_url(body.url.trim()) else {
        return json_error(
            StatusCode::BAD_REQUEST,
            "invalid_url",
            "an http or https address without credentials is required",
        );
    };

    let recipe = AutomationRecipe {
        id: Uuid::new_v4().to_string(),
        kind: AutomationKind::Shortcut,
        name: name.to_owned(),
        target_url,
    };
    if validate_recipe(&recipe).is_err() {
        return json_error(
            StatusCode::BAD_REQUEST,
            "invalid_recipe",
            "the shortcut could not be validated",
        );
    }

    match upsert_automation_recipe(&state.recipes_path, recipe.clone()) {
        Ok(_) => (
            StatusCode::CREATED,
            Json(ShortcutResponse {
                id: recipe.id,
                name: recipe.name,
                target_url: recipe.target_url,
            }),
        )
            .into_response(),
        Err(error) => {
            eprintln!("자동화 브리지가 바로가기를 저장하지 못했습니다: {error}");
            json_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "storage_failed",
                "the shortcut could not be stored",
            )
        }
    }
}

/// Every request needs the pairing token. When a browser sends an `Origin`
/// header it must be the pinned extension id, which keeps web pages out even if
/// they ever learned the token. Local tools (curl, scripts) do not send an
/// `Origin` header, so the token alone is enough for them.
async fn guard(State(state): State<BridgeState>, request: Request, next: Next) -> Response {
    let origin = request
        .headers()
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    if origin.is_some_and(|origin| origin != allowed_origin()) {
        return json_error(
            StatusCode::FORBIDDEN,
            "origin_not_allowed",
            "browser requests must come from the paired extension",
        );
    }

    let token_ok = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(|value| constant_time_eq(value.trim(), state.token.as_str()))
        .unwrap_or(false);
    if !token_ok {
        return json_error(
            StatusCode::UNAUTHORIZED,
            "token_invalid",
            "the pairing token is missing or wrong",
        );
    }

    next.run(request).await
}

pub fn token_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(automation_config_dir(app)?.join(BRIDGE_TOKEN_FILE))
}

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// The pairing token has to be shown in the app UI, so it is stored as plain
/// text inside the user's config directory (owner-only permissions where the
/// platform supports them) rather than in a keychain.
pub fn ensure_token(app: &tauri::AppHandle) -> Result<String, String> {
    let path = token_path(app)?;
    if let Ok(existing) = fs::read_to_string(&path) {
        let trimmed = existing.trim().to_owned();
        if trimmed.len() == 64
            && trimmed
                .chars()
                .all(|character| character.is_ascii_hexdigit())
        {
            return Ok(trimmed);
        }
    }

    let token = generate_token();
    write_file_atomically(&path, &token)?;
    Ok(token)
}

/// Starts the bridge. A failure here must not stop the desktop app: the bridge
/// is a convenience for the extension, not a startup requirement.
pub fn spawn(app: &tauri::AppHandle) -> Result<(), String> {
    let state = BridgeState {
        token: Arc::new(ensure_token(app)?),
        recipes_path: automation_recipes_path(app)?,
    };
    tauri::async_runtime::spawn(async move {
        let router = router(state);
        match TcpListener::bind(("127.0.0.1", BRIDGE_PORT)).await {
            Ok(listener) => {
                if let Err(error) = axum::serve(listener, router).await {
                    eprintln!("로컬 브리지가 멈췄습니다: {error}");
                }
            }
            Err(error) => {
                eprintln!("로컬 브리지를 열지 못했습니다(127.0.0.1:{BRIDGE_PORT}): {error}")
            }
        }
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use std::io::{Read, Write};
    use std::path::PathBuf;
    use std::time::Duration;
    use tower::ServiceExt;

    fn test_token() -> String {
        "a".repeat(64)
    }

    fn test_directory(name: &str) -> PathBuf {
        let directory = std::env::temp_dir().join(format!(
            "school-collect-bridge-{name}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&directory);
        fs::create_dir_all(&directory).unwrap();
        directory
    }

    fn state(directory: &std::path::Path) -> BridgeState {
        BridgeState {
            token: Arc::new(test_token()),
            recipes_path: directory.join(crate::AUTOMATION_RECIPES_FILE),
        }
    }

    fn request(
        method: &str,
        uri: &str,
        token: Option<&str>,
        origin: Option<&str>,
    ) -> Request<Body> {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(token) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {token}"));
        }
        if let Some(origin) = origin {
            builder = builder.header(header::ORIGIN, origin);
        }
        builder.body(Body::empty()).unwrap()
    }

    async fn status_code(app: Router, request: Request<Body>) -> StatusCode {
        app.oneshot(request).await.unwrap().status()
    }

    #[test]
    fn token_comparison_rejects_differences_and_length_mismatch() {
        assert!(constant_time_eq("aaaa", "aaaa"));
        assert!(!constant_time_eq("aaaa", "aaab"));
        assert!(!constant_time_eq("aaaa", "aaa"));
        assert!(!constant_time_eq("", "a"));
    }

    #[tokio::test]
    async fn status_requires_the_pairing_token_and_a_pinned_origin_when_present() {
        let directory = test_directory("guard");
        let app = router(state(&directory));
        let origin = allowed_origin();

        assert_eq!(
            status_code(
                app.clone(),
                request("GET", "/v1/bridge/status", None, Some(&origin))
            )
            .await,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            status_code(
                app.clone(),
                request(
                    "GET",
                    "/v1/bridge/status",
                    Some(&test_token()),
                    Some("chrome-extension://someone-else"),
                ),
            )
            .await,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            status_code(
                app.clone(),
                request("GET", "/v1/bridge/status", Some("wrong"), Some(&origin)),
            )
            .await,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            status_code(
                app.clone(),
                request(
                    "GET",
                    "/v1/bridge/status",
                    Some(&"b".repeat(64)),
                    Some(&origin),
                ),
            )
            .await,
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            status_code(
                app.clone(),
                request(
                    "GET",
                    "/v1/bridge/status",
                    Some(&test_token()),
                    Some(&origin)
                ),
            )
            .await,
            StatusCode::OK
        );
        // A local tool without an Origin header still needs the token.
        assert_eq!(
            status_code(
                app,
                request("GET", "/v1/bridge/status", Some(&test_token()), None),
            )
            .await,
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn preflight_is_answered_for_the_extension_origin() {
        let directory = test_directory("preflight");
        let app = router(state(&directory));
        let response = app
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/v1/bridge/shortcuts")
                    .header(header::ORIGIN, allowed_origin())
                    .header("access-control-request-method", "POST")
                    .header(
                        "access-control-request-headers",
                        "authorization,content-type",
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(response.status().is_success());
        assert_eq!(
            response
                .headers()
                .get("access-control-allow-origin")
                .and_then(|value| value.to_str().ok()),
            Some(allowed_origin().as_str())
        );
    }

    #[tokio::test]
    async fn shortcuts_endpoint_stores_a_recipe() {
        let directory = test_directory("create");
        let app = router(state(&directory));
        let body = serde_json::json!({
            "name": "기안",
            "url": "https://example.invalid/draft"
        });
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/bridge/shortcuts")
                    .header(header::ORIGIN, allowed_origin())
                    .header(header::AUTHORIZATION, format!("Bearer {}", test_token()))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);
        let stored = fs::read_to_string(directory.join(crate::AUTOMATION_RECIPES_FILE)).unwrap();
        assert!(stored.contains("기안"), "stored: {stored}");
        assert!(
            stored.contains("https://example.invalid/draft"),
            "stored: {stored}"
        );
    }

    #[tokio::test]
    async fn shortcuts_endpoint_rejects_bad_input() {
        let directory = test_directory("reject");
        let app = router(state(&directory));

        for payload in [
            serde_json::json!({ "name": "기안", "url": "https://:8080/portal" }),
            serde_json::json!({ "name": "기안", "url": "file:///tmp/x" }),
            serde_json::json!({ "name": "기안", "url": "https://user:pass@example.invalid/" }),
            serde_json::json!({ "name": "   ", "url": "https://example.invalid/draft" }),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/v1/bridge/shortcuts")
                        .header(header::ORIGIN, allowed_origin())
                        .header(header::AUTHORIZATION, format!("Bearer {}", test_token()))
                        .header(header::CONTENT_TYPE, "application/json")
                        .body(Body::from(payload.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(
                response.status(),
                StatusCode::BAD_REQUEST,
                "payload: {payload}"
            );
        }

        assert!(!directory.join(crate::AUTOMATION_RECIPES_FILE).exists());
    }

    #[test]
    fn bridge_serves_over_a_real_socket() {
        let directory = test_directory("socket");
        let app = router(state(&directory));
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
        let address = listener.local_addr().expect("address");

        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("runtime");
            runtime.block_on(async move {
                // tokio requires a non-blocking socket before adopting it.
                listener
                    .set_nonblocking(true)
                    .expect("non-blocking listener");
                let listener = TcpListener::from_std(listener).expect("tokio listener");
                let _ = ready_tx.send(());
                let _ = axum::serve(listener, app).await;
            });
        });
        ready_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("the bridge server should start");

        let connect_deadline = std::time::Instant::now() + Duration::from_secs(5);
        let mut stream = loop {
            match std::net::TcpStream::connect(address) {
                Ok(stream) => break stream,
                Err(_) if std::time::Instant::now() < connect_deadline => {
                    std::thread::sleep(Duration::from_millis(50));
                }
                Err(error) => panic!("connect failed: {error}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("read timeout");
        let request = format!(
            "GET /v1/bridge/status HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: {}\r\nAuthorization: Bearer {}\r\nConnection: close\r\n\r\n",
            allowed_origin(),
            test_token()
        );
        stream.write_all(request.as_bytes()).expect("write");

        // Read until the status line and the JSON body arrive; waiting for EOF
        // would hang whenever the server keeps the connection open.
        let mut response = String::new();
        let mut buffer = [0u8; 512];
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        while std::time::Instant::now() < deadline {
            match stream.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => {
                    response.push_str(&String::from_utf8_lossy(&buffer[..read]));
                    if response.contains("school-collect-automation-bridge") {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        assert!(
            response.starts_with("HTTP/1.1 200"),
            "unexpected response: {response}"
        );
        assert!(response.contains("school-collect-automation-bridge"));
    }
}
