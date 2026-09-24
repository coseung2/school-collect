use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use chrono::{DateTime, Utc};
use school_collect_auth::{AuthMode, OidcVerifier, VerifiedPrincipal};
use school_collect_contracts::{
    ApiError, CollectDetailResponse, CollectDto, CollectListResponse, CreateCollectRequest,
    CreateTenantRequest, MembershipDto, PrincipalResponse, SaveSubmissionRequest, ServiceStatus,
    SessionResponse, SubmissionDto, TenantListResponse, UserDto, VersionConflictResponse,
};
use school_collect_db::{
    CollectRecord, SaveDraftOutcome, SubmitOutcome, TransitionOutcome, UserRecord,
};
use school_collect_domain::MembershipRole;
use sqlx::PgPool;
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::TraceLayer,
};
use utoipa::OpenApi;
use uuid::Uuid;

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
    paths(
        health,
        readiness,
        current_principal,
        session,
        list_tenants,
        create_tenant,
        list_collects,
        create_collect,
        collect_detail,
        publish_collect,
        close_collect,
        save_submission,
        submit_submission
    ),
    components(schemas(
        ServiceStatus,
        ApiError,
        PrincipalResponse,
        UserDto,
        MembershipDto,
        SessionResponse,
        TenantListResponse,
        CreateTenantRequest,
        CollectDto,
        CollectListResponse,
        CreateCollectRequest,
        SubmissionDto,
        CollectDetailResponse,
        SaveSubmissionRequest,
        VersionConflictResponse
    ))
)]
struct ApiDoc;

pub fn router(state: AppState, cors_origin: HeaderValue, auth_state: AuthState) -> Router {
    let protected = Router::new()
        .route("/v1/auth/principal", get(current_principal))
        .route("/v1/session", get(session))
        .route("/v1/tenants", get(list_tenants).post(create_tenant))
        .route("/v1/collects", get(list_collects).post(create_collect))
        .route("/v1/collects/{collect_id}", get(collect_detail))
        .route("/v1/collects/{collect_id}/publish", post(publish_collect))
        .route("/v1/collects/{collect_id}/close", post(close_collect))
        .route("/v1/collects/{collect_id}/submission", put(save_submission))
        .route(
            "/v1/collects/{collect_id}/submission/submit",
            post(submit_submission),
        )
        .route_layer(middleware::from_fn_with_state(
            auth_state.clone(),
            authenticate,
        ));

    Router::new()
        .route("/health", get(health))
        .route("/ready", get(readiness))
        .route("/openapi.json", get(|| async { Json(ApiDoc::openapi()) }))
        .merge(protected)
        .with_state(Arc::new(state))
        .layer(DefaultBodyLimit::max(256 * 1024))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(
            CorsLayer::new()
                .allow_origin(cors_origin)
                .allow_methods([Method::GET, Method::POST, Method::PUT])
                .allow_headers([
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderName::from_static("x-tenant-id"),
                    axum::http::HeaderName::from_static("apikey"),
                ]),
        )
        .layer(TraceLayer::new_for_http())
}

fn request_id_of(headers: &HeaderMap) -> String {
    headers
        .get("x-request-id")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("request-untracked")
        .to_owned()
}

/// A failure that already knows the request id, used inside authenticated
/// handlers where the header map is available.
fn failure(
    headers: &HeaderMap,
    status: StatusCode,
    code: &str,
    message: impl Into<String>,
) -> Response {
    (
        status,
        Json(ApiError {
            code: code.to_owned(),
            message: message.into(),
            request_id: request_id_of(headers),
        }),
    )
        .into_response()
}

fn conflict_response(headers: &HeaderMap, current_version: i64) -> Response {
    (
        StatusCode::CONFLICT,
        Json(VersionConflictResponse {
            code: "version_conflict".to_owned(),
            message: "the draft was changed by another device; reload before saving".to_owned(),
            request_id: request_id_of(headers),
            current_version,
        }),
    )
        .into_response()
}

async fn authenticate(State(auth): State<AuthState>, mut request: Request, next: Next) -> Response {
    let principal = match auth.mode {
        AuthMode::DisabledForDevelopment => {
            // Development-only identity. It is unreachable outside APP_ENV=development
            // because AuthMode::from_env refuses to build this variant there.
            VerifiedPrincipal {
                issuer: "development".to_owned(),
                subject: "local-development".to_owned(),
                email: Some("dev@localhost".to_owned()),
            }
        }
        AuthMode::Oidc => {
            let Some(verifier) = auth.verifier else {
                return failure(
                    request.headers(),
                    StatusCode::SERVICE_UNAVAILABLE,
                    "authentication_unavailable",
                    "authentication is not configured",
                );
            };
            let Some(header) = request.headers().get("authorization") else {
                return failure(
                    request.headers(),
                    StatusCode::UNAUTHORIZED,
                    "unauthorized",
                    "a bearer access token is required",
                );
            };
            let Ok(header) = header.to_str() else {
                return failure(
                    request.headers(),
                    StatusCode::UNAUTHORIZED,
                    "unauthorized",
                    "the authorization header is not valid text",
                );
            };
            match verifier.verify(header).await {
                Ok(principal) => principal,
                Err(error) => {
                    // The reason is logged for operators; the token itself never is.
                    tracing::warn!(reason = %error, "access token verification failed");
                    return failure(
                        request.headers(),
                        StatusCode::UNAUTHORIZED,
                        "unauthorized",
                        "the access token could not be verified",
                    );
                }
            }
        }
    };

    request.extensions_mut().insert(principal);
    next.run(request).await
}

/// Resolves the verified identity to a local user row.
///
/// The row is created on first sight of a verified issuer/subject pair, so a
/// client can never choose its own user id.
async fn current_user(
    state: &AppState,
    principal: &VerifiedPrincipal,
    headers: &HeaderMap,
) -> Result<UserRecord, Box<Response>> {
    let display_name = principal
        .email
        .as_deref()
        .map(|email| email.split('@').next().unwrap_or(email))
        .unwrap_or(&principal.subject);
    school_collect_db::upsert_user(
        &state.pool,
        &principal.issuer,
        &principal.subject,
        Some(display_name),
    )
    .await
    .map_err(|_| Box::new(storage_failure(headers)))
}

fn tenant_id_from(headers: &HeaderMap) -> Result<Uuid, Box<Response>> {
    let raw = headers
        .get("x-tenant-id")
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| {
            Box::new(failure(
                headers,
                StatusCode::BAD_REQUEST,
                "tenant_required",
                "the X-Tenant-Id header is required",
            ))
        })?;
    Uuid::parse_str(raw).map_err(|_| {
        Box::new(failure(
            headers,
            StatusCode::BAD_REQUEST,
            "tenant_invalid",
            "the X-Tenant-Id header is not a valid identifier",
        ))
    })
}

/// Server-side authorization. A tenant id supplied by the client is only ever
/// an input to this lookup; it never grants access on its own.
async fn authorize(
    state: &AppState,
    headers: &HeaderMap,
    user: &UserRecord,
    capability: Capability,
) -> Result<(Uuid, MembershipRole), Box<Response>> {
    let tenant_id = tenant_id_from(headers)?;
    let role = school_collect_db::membership_role(&state.pool, tenant_id, user.id)
        .await
        .map_err(|_| {
            Box::new(failure(
                headers,
                StatusCode::INTERNAL_SERVER_ERROR,
                "storage_unavailable",
                "the request could not be completed",
            ))
        })?
        .ok_or_else(|| {
            Box::new(failure(
                headers,
                StatusCode::FORBIDDEN,
                "forbidden",
                "you do not have access to this school",
            ))
        })?;

    let parsed = MembershipRole::parse(&role).ok_or_else(|| {
        Box::new(failure(
            headers,
            StatusCode::FORBIDDEN,
            "forbidden",
            "your membership does not grant a usable role",
        ))
    })?;

    let allowed = match capability {
        Capability::View => true,
        Capability::Submit => parsed.can_submit(),
        Capability::Manage => parsed.can_manage_collects(),
    };
    if !allowed {
        return Err(Box::new(failure(
            headers,
            StatusCode::FORBIDDEN,
            "forbidden",
            "your role does not allow this action",
        )));
    }

    Ok((tenant_id, parsed))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Capability {
    View,
    Submit,
    Manage,
}

fn storage_failure(headers: &HeaderMap) -> Response {
    failure(
        headers,
        StatusCode::INTERNAL_SERVER_ERROR,
        "storage_unavailable",
        "the request could not be completed",
    )
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}

fn collect_dto(record: &CollectRecord) -> CollectDto {
    CollectDto {
        id: record.id.to_string(),
        title: record.title.clone(),
        status: record.status.clone(),
        due_at: record.due_at.map(timestamp),
        version: record.version,
        updated_at: timestamp(record.updated_at),
        submission_status: None,
        submission_version: None,
    }
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
    let ready = school_collect_db::ping(&state.pool).await.is_ok()
        && school_collect_db::verify_schema(&state.pool).await.is_ok();

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
    path = "/v1/auth/principal",
    responses(
        (status = 200, description = "Verified principal", body = PrincipalResponse),
        (status = 401, description = "Authentication failed", body = ApiError)
    )
)]
async fn current_principal(
    Extension(principal): Extension<VerifiedPrincipal>,
) -> Json<PrincipalResponse> {
    Json(PrincipalResponse {
        issuer: principal.issuer,
        subject: principal.subject,
        email: principal.email,
    })
}

#[utoipa::path(
    get,
    path = "/v1/session",
    responses(
        (status = 200, description = "Caller identity and school memberships", body = SessionResponse),
        (status = 401, description = "Authentication failed", body = ApiError)
    )
)]
async fn session(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
) -> Response {
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let memberships = match school_collect_db::list_memberships(&state.pool, user.id).await {
        Ok(rows) => rows,
        Err(_) => return storage_failure(&headers),
    };

    Json(SessionResponse {
        user: UserDto {
            id: user.id.to_string(),
            issuer: user.issuer,
            subject: user.subject,
            display_name: user.display_name,
        },
        memberships: memberships
            .into_iter()
            .map(|row| MembershipDto {
                tenant_id: row.tenant_id.to_string(),
                tenant_name: row.tenant_name,
                role: row.role,
            })
            .collect(),
    })
    .into_response()
}

#[utoipa::path(
    get,
    path = "/v1/tenants",
    responses((status = 200, description = "Schools the caller belongs to", body = TenantListResponse))
)]
async fn list_tenants(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
) -> Response {
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    match school_collect_db::list_memberships(&state.pool, user.id).await {
        Ok(rows) => Json(TenantListResponse {
            tenants: rows
                .into_iter()
                .map(|row| MembershipDto {
                    tenant_id: row.tenant_id.to_string(),
                    tenant_name: row.tenant_name,
                    role: row.role,
                })
                .collect(),
        })
        .into_response(),
        Err(_) => storage_failure(&headers),
    }
}

#[utoipa::path(
    post,
    path = "/v1/tenants",
    request_body = CreateTenantRequest,
    responses((status = 201, description = "School created", body = MembershipDto))
)]
async fn create_tenant(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
    Json(body): Json<CreateTenantRequest>,
) -> Response {
    let name = body.name.trim();
    if name.is_empty() || name.chars().count() > 120 {
        return failure(
            &headers,
            StatusCode::BAD_REQUEST,
            "invalid_name",
            "a school name between 1 and 120 characters is required",
        );
    }
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    match school_collect_db::create_tenant(&state.pool, user.id, name).await {
        Ok(tenant) => (
            StatusCode::CREATED,
            Json(MembershipDto {
                tenant_id: tenant.id.to_string(),
                tenant_name: tenant.name,
                role: "admin".to_owned(),
            }),
        )
            .into_response(),
        Err(_) => storage_failure(&headers),
    }
}

#[utoipa::path(
    get,
    path = "/v1/collects",
    responses((status = 200, description = "Collects visible to the caller", body = CollectListResponse))
)]
async fn list_collects(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
) -> Response {
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (tenant_id, _) = match authorize(&state, &headers, &user, Capability::View).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match school_collect_db::list_collects(&state.pool, tenant_id, user.id).await {
        Ok(rows) => Json(CollectListResponse {
            collects: rows
                .into_iter()
                .map(|row| CollectDto {
                    id: row.id.to_string(),
                    title: row.title,
                    status: row.status,
                    due_at: row.due_at.map(timestamp),
                    version: row.version,
                    updated_at: timestamp(row.updated_at),
                    submission_status: row.submission_status,
                    submission_version: row.submission_version,
                })
                .collect(),
        })
        .into_response(),
        Err(_) => storage_failure(&headers),
    }
}

#[utoipa::path(
    post,
    path = "/v1/collects",
    request_body = CreateCollectRequest,
    responses((status = 201, description = "Draft collect created", body = CollectDto))
)]
async fn create_collect(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
    Json(body): Json<CreateCollectRequest>,
) -> Response {
    let title = body.title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return failure(
            &headers,
            StatusCode::BAD_REQUEST,
            "invalid_title",
            "a title between 1 and 200 characters is required",
        );
    }
    let due_at = match body.due_at.as_deref() {
        None => None,
        Some(raw) => match DateTime::parse_from_rfc3339(raw) {
            Ok(value) => Some(value.with_timezone(&Utc)),
            Err(_) => {
                return failure(
                    &headers,
                    StatusCode::BAD_REQUEST,
                    "invalid_due_at",
                    "dueAt must be an RFC 3339 timestamp",
                );
            }
        },
    };

    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (tenant_id, _) = match authorize(&state, &headers, &user, Capability::Manage).await {
        Ok(value) => value,
        Err(response) => return *response,
    };

    match school_collect_db::create_collect(
        &state.pool,
        tenant_id,
        user.id,
        title,
        body.description.trim(),
        due_at,
    )
    .await
    {
        Ok(record) => (StatusCode::CREATED, Json(collect_dto(&record))).into_response(),
        Err(_) => storage_failure(&headers),
    }
}

#[utoipa::path(
    get,
    path = "/v1/collects/{collect_id}",
    params(("collect_id" = String, Path, description = "Collect identifier")),
    responses((status = 200, description = "Collect detail", body = CollectDetailResponse))
)]
async fn collect_detail(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
    Path(collect_id): Path<String>,
) -> Response {
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (tenant_id, _) = match authorize(&state, &headers, &user, Capability::View).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let Ok(collect_id) = Uuid::parse_str(&collect_id) else {
        return failure(
            &headers,
            StatusCode::BAD_REQUEST,
            "invalid_id",
            "the collect identifier is not valid",
        );
    };

    let record = match school_collect_db::get_collect(&state.pool, tenant_id, collect_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return failure(
                &headers,
                StatusCode::NOT_FOUND,
                "not_found",
                "the collect was not found in this school",
            );
        }
        Err(_) => return storage_failure(&headers),
    };

    let submission = match school_collect_db::get_submission(
        &state.pool,
        tenant_id,
        collect_id,
        user.id,
    )
    .await
    {
        Ok(value) => value.map(|record| SubmissionDto {
            status: record.status,
            version: record.version,
            payload: record.payload,
            submitted_at: record.submitted_at.map(timestamp),
            updated_at: timestamp(record.updated_at),
        }),
        Err(_) => return storage_failure(&headers),
    };

    Json(CollectDetailResponse {
        id: record.id.to_string(),
        title: record.title,
        description: record.description,
        status: record.status,
        due_at: record.due_at.map(timestamp),
        version: record.version,
        updated_at: timestamp(record.updated_at),
        submission,
    })
    .into_response()
}

async fn transition(
    state: &AppState,
    headers: &HeaderMap,
    principal: &VerifiedPrincipal,
    collect_id: &str,
    allowed_from: &[&str],
    to: &str,
) -> Response {
    let user = match current_user(state, principal, headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (tenant_id, _) = match authorize(state, headers, &user, Capability::Manage).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let Ok(collect_id) = Uuid::parse_str(collect_id) else {
        return failure(
            headers,
            StatusCode::BAD_REQUEST,
            "invalid_id",
            "the collect identifier is not valid",
        );
    };

    match school_collect_db::transition_collect(
        &state.pool,
        tenant_id,
        collect_id,
        allowed_from,
        to,
        user.id,
    )
    .await
    {
        Ok(TransitionOutcome::Changed(record)) => {
            (StatusCode::OK, Json(collect_dto(&record))).into_response()
        }
        Ok(TransitionOutcome::InvalidState { status }) => failure(
            headers,
            StatusCode::CONFLICT,
            "invalid_state_transition",
            format!("a collect in state {status} cannot move to {to}"),
        ),
        Ok(TransitionOutcome::NotFound) => failure(
            headers,
            StatusCode::NOT_FOUND,
            "not_found",
            "the collect was not found in this school",
        ),
        Err(_) => storage_failure(headers),
    }
}

#[utoipa::path(
    post,
    path = "/v1/collects/{collect_id}/publish",
    params(("collect_id" = String, Path, description = "Collect identifier")),
    responses((status = 200, description = "Collect published", body = CollectDto))
)]
async fn publish_collect(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
    Path(collect_id): Path<String>,
) -> Response {
    transition(
        &state,
        &headers,
        &principal,
        &collect_id,
        &["draft"],
        "published",
    )
    .await
}

#[utoipa::path(
    post,
    path = "/v1/collects/{collect_id}/close",
    params(("collect_id" = String, Path, description = "Collect identifier")),
    responses((status = 200, description = "Collect closed", body = CollectDto))
)]
async fn close_collect(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
    Path(collect_id): Path<String>,
) -> Response {
    transition(
        &state,
        &headers,
        &principal,
        &collect_id,
        &["published"],
        "closed",
    )
    .await
}

#[utoipa::path(
    put,
    path = "/v1/collects/{collect_id}/submission",
    params(("collect_id" = String, Path, description = "Collect identifier")),
    request_body = SaveSubmissionRequest,
    responses(
        (status = 200, description = "Draft saved", body = SubmissionDto),
        (status = 409, description = "Version conflict", body = VersionConflictResponse)
    )
)]
async fn save_submission(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
    Path(collect_id): Path<String>,
    Json(body): Json<SaveSubmissionRequest>,
) -> Response {
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (tenant_id, _) = match authorize(&state, &headers, &user, Capability::Submit).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let Ok(collect_id) = Uuid::parse_str(&collect_id) else {
        return failure(
            &headers,
            StatusCode::BAD_REQUEST,
            "invalid_id",
            "the collect identifier is not valid",
        );
    };

    match school_collect_db::save_draft(
        &state.pool,
        tenant_id,
        collect_id,
        user.id,
        body.expected_version,
        &body.payload,
    )
    .await
    {
        Ok(SaveDraftOutcome::Saved(record)) => (
            StatusCode::OK,
            Json(SubmissionDto {
                status: record.status,
                version: record.version,
                payload: record.payload,
                submitted_at: record.submitted_at.map(timestamp),
                updated_at: timestamp(record.updated_at),
            }),
        )
            .into_response(),
        Ok(SaveDraftOutcome::VersionConflict { current_version }) => {
            conflict_response(&headers, current_version)
        }
        Ok(SaveDraftOutcome::AlreadySubmitted) => failure(
            &headers,
            StatusCode::CONFLICT,
            "already_submitted",
            "this submission was already sent and can no longer be edited",
        ),
        Ok(SaveDraftOutcome::CollectNotOpen { status }) => failure(
            &headers,
            StatusCode::CONFLICT,
            "collect_not_open",
            format!("a collect in state {status} does not accept submissions"),
        ),
        Err(_) => storage_failure(&headers),
    }
}

#[utoipa::path(
    post,
    path = "/v1/collects/{collect_id}/submission/submit",
    params(("collect_id" = String, Path, description = "Collect identifier")),
    responses((status = 200, description = "Submission sent", body = SubmissionDto))
)]
async fn submit_submission(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
    Path(collect_id): Path<String>,
) -> Response {
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (tenant_id, _) = match authorize(&state, &headers, &user, Capability::Submit).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let Ok(collect_id) = Uuid::parse_str(&collect_id) else {
        return failure(
            &headers,
            StatusCode::BAD_REQUEST,
            "invalid_id",
            "the collect identifier is not valid",
        );
    };

    match school_collect_db::submit(&state.pool, tenant_id, collect_id, user.id).await {
        Ok(SubmitOutcome::Submitted(record)) => (
            StatusCode::OK,
            Json(SubmissionDto {
                status: record.status,
                version: record.version,
                payload: record.payload,
                submitted_at: record.submitted_at.map(timestamp),
                updated_at: timestamp(record.updated_at),
            }),
        )
            .into_response(),
        Ok(SubmitOutcome::AlreadySubmitted) => failure(
            &headers,
            StatusCode::CONFLICT,
            "already_submitted",
            "this submission was already sent",
        ),
        Ok(SubmitOutcome::NothingToSubmit) => failure(
            &headers,
            StatusCode::CONFLICT,
            "nothing_to_submit",
            "save a draft before sending it",
        ),
        Ok(SubmitOutcome::CollectNotOpen { status }) => failure(
            &headers,
            StatusCode::CONFLICT,
            "collect_not_open",
            format!("a collect in state {status} does not accept submissions"),
        ),
        Err(_) => storage_failure(&headers),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
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
            ("OIDC_AUDIENCE", "authenticated"),
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

    #[tokio::test]
    async fn oidc_mode_rejects_a_forged_token() {
        let values = HashMap::from([
            ("OIDC_ISSUER_URL", "https://id.example.test"),
            ("OIDC_AUDIENCE", "authenticated"),
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
                .uri("/v1/session")
                .header("authorization", "Bearer not.a.jwt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
