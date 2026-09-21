use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use school_collect_contracts::ServiceStatus;
use sqlx::PgPool;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use utoipa::OpenApi;

#[derive(Clone, Default)]
pub struct AppState {
    pub pool: Option<PgPool>,
}

#[derive(OpenApi)]
#[openapi(
    paths(health, readiness),
    components(schemas(ServiceStatus))
)]
struct ApiDoc;

pub fn router(state: AppState, cors_origin: HeaderValue) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ready", get(readiness))
        .route("/openapi.json", get(|| async { Json(ApiDoc::openapi()) }))
        .with_state(Arc::new(state))
        .layer(
            CorsLayer::new()
                .allow_origin(cors_origin)
                .allow_methods([Method::GET]),
        )
        .layer(TraceLayer::new_for_http())
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
    let ready = match &state.pool {
        Some(pool) => school_collect_db::ping(pool).await.is_ok(),
        None => false,
    };

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
    use tower::ServiceExt;

    #[tokio::test]
    async fn health_does_not_depend_on_postgres() {
        let response = router(
            AppState::default(),
            HeaderValue::from_static("http://127.0.0.1:1420"),
        )
        .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn readiness_fails_without_postgres() {
        let response = router(
            AppState::default(),
            HeaderValue::from_static("http://127.0.0.1:1420"),
        )
        .oneshot(Request::builder().uri("/ready").body(Body::empty()).unwrap())
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
