use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use school_collect_auth::{Principal, TokenVerifier};
use school_collect_contracts::ServiceStatus;
use serde::Serialize;
use sqlx::PgPool;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa::OpenApi;

/// Shared request state.
///
/// The pool is created eagerly from configuration but connects lazily, so a
/// temporarily unreachable database is a readiness problem rather than a
/// startup problem.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

#[derive(OpenApi)]
#[openapi(
    paths(health, readiness, session),
    components(schemas(ServiceStatus, SessionStatus))
)]
struct ApiDoc;

pub fn router(
    state: AppState,
    cors_origin: HeaderValue,
    auth: Option<Arc<TokenVerifier>>,
) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(readiness))
        .route("/openapi.json", get(|| async { Json(ApiDoc::openapi()) }))
        .route(
            "/v1/session",
            get(session).route_layer(middleware::from_fn_with_state(auth, authenticate)),
        )
        .with_state(Arc::new(state))
        .layer(
            CorsLayer::new()
                .allow_origin(cors_origin)
                .allow_methods([Method::GET]),
        )
        .layer(TraceLayer::new_for_http())
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SessionStatus {
    pub subject: String,
    pub expires_at: u64,
}

async fn authenticate(
    State(verifier): State<Option<Arc<TokenVerifier>>>,
    mut request: Request,
    next: Next,
) -> Response {
    let Some(verifier) = verifier else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };

    let Some(authorization) = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
    else {
        return unauthorized();
    };

    match verifier.verify_bearer(authorization).await {
        Ok(principal) => {
            request.extensions_mut().insert(principal);
            next.run(request).await
        }
        Err(_) => unauthorized(),
    }
}

fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [(axum::http::header::WWW_AUTHENTICATE, "Bearer")],
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "Process is alive", body = ServiceStatus))
)]
async fn health() -> Json<ServiceStatus> {
    Json(ServiceStatus {
        status: "ok",
        service: "api",
    })
}

#[utoipa::path(
    get,
    path = "/ready",
    responses(
        (status = 200, description = "Dependencies are ready", body = ServiceStatus),
        (status = 503, description = "A required dependency is unavailable", body = ServiceStatus)
    )
)]
async fn readiness(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let ready = school_collect_db::ping(&state.pool).await.is_ok();

    let body = ServiceStatus {
        status: if ready { "ready" } else { "not_ready" },
        service: "api",
    };

    if ready {
        (StatusCode::OK, Json(body))
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, Json(body))
    }
}

#[utoipa::path(
    get,
    path = "/v1/session",
    responses(
        (status = 200, description = "Authenticated session", body = SessionStatus),
        (status = 401, description = "Missing or invalid access token"),
        (status = 503, description = "Authentication is not configured")
    )
)]
async fn session(Extension(principal): Extension<Principal>) -> Json<SessionStatus> {
    Json(SessionStatus {
        subject: principal.subject,
        expires_at: principal.expires_at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use tower::ServiceExt;

    /// Points at a closed loopback port so readiness fails without needing a
    /// live database or any seeded data.
    fn unreachable_state() -> AppState {
        AppState {
            pool: school_collect_db::lazy_pool(
                "postgres://unused:unused@127.0.0.1:1/school_collect_absent",
            )
            .expect("connection string is valid"),
        }
    }

    #[tokio::test]
    async fn health_does_not_depend_on_postgres() {
        let response = router(
            unreachable_state(),
            HeaderValue::from_static("http://127.0.0.1:1420"),
            None,
        )
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn readiness_fails_without_postgres() {
        let response = router(
            unreachable_state(),
            HeaderValue::from_static("http://127.0.0.1:1420"),
            None,
        )
        .oneshot(
            Request::builder()
                .uri("/ready")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    #[tokio::test]
    async fn protected_session_rejects_missing_token() {
        let response = router(
            unreachable_state(),
            HeaderValue::from_static("http://127.0.0.1:1420"),
            Some(Arc::new(
                school_collect_auth::TokenVerifier::new(oidc_config()).unwrap(),
            )),
        )
        .oneshot(
            Request::builder()
                .uri("/v1/session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn protected_session_fails_closed_when_auth_is_not_configured() {
        let response = router(
            unreachable_state(),
            HeaderValue::from_static("http://127.0.0.1:1420"),
            None,
        )
        .oneshot(
            Request::builder()
                .uri("/v1/session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }

    fn oidc_config() -> school_collect_auth::OidcConfig {
        school_collect_auth::OidcConfig {
            issuer: "https://id.example.test".to_owned(),
            audience: "school-collect-api".to_owned(),
            jwks_url: "https://id.example.test/oauth/keys".parse().unwrap(),
        }
    }
}
