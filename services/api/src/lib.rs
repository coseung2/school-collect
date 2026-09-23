use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use school_collect_auth::{AuthMode, OidcVerifier, VerifiedPrincipal};
use school_collect_contracts::{ApiError, PrincipalResponse, ServiceStatus};
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

#[derive(Clone)]
pub struct AuthState {
    pub mode: AuthMode,
    pub verifier: Option<Arc<OidcVerifier>>,
}

impl AuthState {
    pub fn development_disabled() -> Self {
        Self {
            mode: AuthMode::DisabledForDevelopment,
            verifier: None,
        }
    }

    pub fn oidc(verifier: Arc<OidcVerifier>) -> Self {
        Self {
            mode: AuthMode::Oidc,
            verifier: Some(verifier),
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(health, readiness, current_principal),
    components(schemas(ServiceStatus, PrincipalResponse, ApiError))
)]
struct ApiDoc;

pub fn router(state: AppState, cors_origin: HeaderValue, auth_state: AuthState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(readiness))
        .route("/openapi.json", get(|| async { Json(ApiDoc::openapi()) }))
        .route(
            "/v1/auth/principal",
            get(current_principal).route_layer(middleware::from_fn_with_state(
                auth_state.clone(),
                authenticate,
            )),
        )
        .with_state(Arc::new(state))
        .layer(
            CorsLayer::new()
                .allow_origin(cors_origin)
                .allow_methods([Method::GET]),
        )
        .layer(TraceLayer::new_for_http())
}

async fn authenticate(State(auth): State<AuthState>, mut request: Request, next: Next) -> Response {
    let principal = match auth.mode {
        AuthMode::DisabledForDevelopment => VerifiedPrincipal {
            issuer: "development".to_owned(),
            subject: "local-development".to_owned(),
        },
        AuthMode::Oidc => {
            let Some(verifier) = auth.verifier else {
                return auth_error(&request, "authentication_unavailable");
            };
            let Some(header) = request.headers().get("authorization") else {
                return auth_error(&request, "unauthorized");
            };
            let Ok(header) = header.to_str() else {
                return auth_error(&request, "unauthorized");
            };
            match verifier.verify(header).await {
                Ok(principal) => principal,
                Err(_) => return auth_error(&request, "unauthorized"),
            }
        }
    };

    request.extensions_mut().insert(principal);
    next.run(request).await
}

fn auth_error(request: &Request, code: &'static str) -> Response {
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("request-untracked")
        .to_owned();
    let status = if code == "unauthorized" {
        StatusCode::UNAUTHORIZED
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(ApiError {
            code: code.to_owned(),
            message: "request authentication failed".to_owned(),
            request_id,
        }),
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/v1/auth/principal",
    responses(
        (status = 200, description = "Verified OIDC principal", body = PrincipalResponse),
        (status = 401, description = "Authentication failed", body = ApiError)
    )
)]
async fn current_principal(
    Extension(principal): Extension<VerifiedPrincipal>,
) -> Json<PrincipalResponse> {
    Json(PrincipalResponse {
        issuer: principal.issuer,
        subject: principal.subject,
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use school_collect_auth::{OidcConfig, OidcVerifier};
    use std::collections::HashMap;
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
            AuthState::development_disabled(),
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
            AuthState::development_disabled(),
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
    async fn development_auth_exposes_a_verified_local_principal() {
        let response = router(
            unreachable_state(),
            HeaderValue::from_static("http://127.0.0.1:1420"),
            AuthState::development_disabled(),
        )
        .oneshot(
            Request::builder()
                .uri("/v1/auth/principal")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn oidc_mode_rejects_requests_without_an_authorization_header() {
        let values = HashMap::from([
            ("OIDC_ISSUER_URL", "https://id.example.test"),
            ("OIDC_AUDIENCE", "school-collect-api"),
        ]);
        let config =
            OidcConfig::from_env(|key| values.get(key).map(|value| (*value).to_owned())).unwrap();
        let verifier = OidcVerifier::new(config).unwrap();
        let response = router(
            unreachable_state(),
            HeaderValue::from_static("http://127.0.0.1:1420"),
            AuthState::oidc(verifier),
        )
        .oneshot(
            Request::builder()
                .uri("/v1/auth/principal")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
