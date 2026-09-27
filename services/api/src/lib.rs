use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post, put},
};
use chrono::{DateTime, Duration, Utc};
use school_collect_application::storage::{ObjectStorage, StorageError};
use school_collect_auth::{AuthMode, OidcVerifier, VerifiedPrincipal};
use school_collect_contracts::{
    AcceptInvitationRequest, AcceptInvitationResponse, ApiError, AssignmentDto,
    AssignmentListResponse, AttachmentDto, AttachmentListResponse, AttachmentUploadTarget,
    CollectDetailResponse, CollectDto, CollectItemDto, CollectListResponse, CollectProgressDto,
    CollectStatusResponse, CollectStatusRowDto, CreateAttachmentRequest, CreateAttachmentResponse,
    CreateCollectRequest, CreateInvitationRequest, CreateInvitationResponse, CreateTenantRequest,
    InvitationDto, InvitationListResponse, MemberDto, MemberListResponse, MembershipDto,
    PrincipalResponse, SaveSubmissionRequest, ServiceStatus, SessionResponse, SubmissionDto,
    TenantListResponse, UpdateCollectAssignmentsRequest, UpdateCollectItemsRequest, UserDto,
    VersionConflictResponse,
};
use school_collect_db::{
    AcceptInvitationOutcome, AttachmentRecord, CollectRecord, CompleteAttachmentOutcome,
    CreateAttachmentOutcome, CreateInvitationOutcome, DeleteAttachmentOutcome, InvitationRecord,
    NewAttachment, NewCollectItem, SaveDraftOutcome, SubmitOutcome, TransitionOutcome,
    UpdateAssignmentsOutcome, UpdateItemsOutcome, UserRecord,
};
use school_collect_domain::{
    MembershipRole,
    attachments::{self, AttachmentRejection, MAX_ATTACHMENT_BYTES},
};
use school_collect_observability::Metrics;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower_http::{
    cors::CorsLayer,
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer},
    trace::{MakeSpan, TraceLayer},
};
use utoipa::OpenApi;
use uuid::Uuid;

pub mod storage;
pub use storage::FileStorage;

/// Shared request state.
///
/// The pool is created eagerly from configuration but connects lazily, so a
/// temporarily unreachable database is a readiness problem rather than a
/// startup problem.
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    /// Private object storage for attachments. `None` means the feature is not
    /// configured and the attachment routes must refuse instead of degrading.
    pub storage: Option<Arc<dyn ObjectStorage>>,
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
        collect_status,
        list_members,
        list_invitations,
        create_invitation,
        revoke_invitation,
        accept_invitation,
        list_assignments,
        publish_collect,
        close_collect,
        save_submission,
        submit_submission,
        create_attachment,
        list_collect_attachments,
        attachment_metadata,
        delete_attachment
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
        InvitationDto,
        InvitationListResponse,
        CreateInvitationRequest,
        CreateInvitationResponse,
        AcceptInvitationRequest,
        AcceptInvitationResponse,
        CollectDto,
        CollectListResponse,
        CreateCollectRequest,
        SubmissionDto,
        CollectDetailResponse,
        CollectItemDto,
        CollectProgressDto,
        CollectStatusResponse,
        CollectStatusRowDto,
        MemberDto,
        MemberListResponse,
        AssignmentDto,
        AssignmentListResponse,
        SaveSubmissionRequest,
        VersionConflictResponse,
        AttachmentDto,
        AttachmentListResponse,
        AttachmentUploadTarget,
        CreateAttachmentRequest,
        CreateAttachmentResponse
    ))
)]
struct ApiDoc;

pub fn router(state: AppState, cors_origin: HeaderValue, auth_state: AuthState) -> Router {
    let protected = Router::new()
        .route("/v1/auth/principal", get(current_principal))
        .route("/v1/session", get(session))
        .route("/v1/tenants", get(list_tenants).post(create_tenant))
        .route(
            "/v1/invitations",
            get(list_invitations).post(create_invitation),
        )
        .route(
            "/v1/invitations/{invitation_id}/revoke",
            post(revoke_invitation),
        )
        .route("/v1/invitations/accept", post(accept_invitation))
        .route("/v1/collects", get(list_collects).post(create_collect))
        .route("/v1/collects/{collect_id}", get(collect_detail))
        .route("/v1/collects/{collect_id}/status", get(collect_status))
        .route("/v1/collects/{collect_id}/export", get(export_collect))
        .route(
            "/v1/collects/{collect_id}/attachments",
            get(list_collect_attachments).post(create_attachment),
        )
        .route(
            "/v1/attachments/{attachment_id}",
            get(attachment_metadata).delete(delete_attachment),
        )
        .route(
            "/v1/attachments/{attachment_id}/content",
            get(download_attachment)
                .put(upload_attachment_content)
                // Bytes are larger than the JSON default, and the slot already
                // promised a size, so the route raises its own limit.
                .layer(DefaultBodyLimit::max(MAX_ATTACHMENT_BYTES as usize + 1)),
        )
        .route("/v1/members", get(list_members))
        .route("/v1/assignments", get(list_assignments))
        .route("/v1/collects/{collect_id}/publish", post(publish_collect))
        .route("/v1/collects/{collect_id}/close", post(close_collect))
        .route("/v1/collects/{collect_id}/items", put(update_collect_items))
        .route(
            "/v1/collects/{collect_id}/assignments",
            put(update_collect_assignments),
        )
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
        .route("/metrics", get(metrics))
        .route("/openapi.json", get(|| async { Json(ApiDoc::openapi()) }))
        .merge(protected)
        .with_state(Arc::new(state))
        .layer(DefaultBodyLimit::max(256 * 1024))
        // Order matters: the request id is assigned first so the tracing span
        // and the metrics middleware can report it.
        .layer(TraceLayer::new_for_http().make_span_with(RequestSpan))
        .layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(middleware::from_fn(record_metrics))
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
}

/// Span for every request, including the request id that the client sees.
#[derive(Clone, Copy)]
struct RequestSpan;

impl<B> MakeSpan<B> for RequestSpan {
    fn make_span(&mut self, request: &axum::http::Request<B>) -> tracing::Span {
        let request_id = request
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .unwrap_or("request-untracked");
        tracing::info_span!(
            "http",
            method = %request.method(),
            path = %request.uri().path(),
            request_id = %request_id,
        )
    }
}

/// Counts every response by status class so operators can alert on 4xx/5xx.
async fn record_metrics(request: Request, next: Next) -> Response {
    let response = next.run(request).await;
    Metrics::global().record_response(response.status().as_u16());
    response
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
                Metrics::global().record_auth_failure();
                return failure(
                    request.headers(),
                    StatusCode::UNAUTHORIZED,
                    "unauthorized",
                    "a bearer access token is required",
                );
            };
            let Ok(header) = header.to_str() else {
                Metrics::global().record_auth_failure();
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
                    Metrics::global().record_auth_failure();
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
            Metrics::global().record_storage_failure();
            Box::new(failure(
                headers,
                StatusCode::INTERNAL_SERVER_ERROR,
                "storage_unavailable",
                "the request could not be completed",
            ))
        })?
        .ok_or_else(|| {
            Metrics::global().record_authorization_denial();
            Box::new(failure(
                headers,
                StatusCode::FORBIDDEN,
                "forbidden",
                "you do not have access to this school",
            ))
        })?;

    let parsed = MembershipRole::parse(&role).ok_or_else(|| {
        Metrics::global().record_authorization_denial();
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
        Metrics::global().record_authorization_denial();
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

fn attachment_dto(record: &AttachmentRecord) -> AttachmentDto {
    AttachmentDto {
        id: record.id.to_string(),
        collect_id: record.collect_id.to_string(),
        user_id: record.user_id.to_string(),
        item_key: record.item_key.clone(),
        file_name: record.file_name.clone(),
        content_type: record.content_type.clone(),
        byte_size: record.byte_size,
        status: record.status.clone(),
        expires_at: timestamp(record.expires_at),
        stored_at: record.stored_at.map(timestamp),
        created_at: timestamp(record.created_at),
        content_url: format!("/v1/attachments/{}/content", record.id),
    }
}

/// Storage is configuration, not a runtime mode: without it the feature is
/// refused and the client is told so.
fn attachment_storage(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Arc<dyn ObjectStorage>, Box<Response>> {
    state.storage.clone().ok_or_else(|| {
        Box::new(failure(
            headers,
            StatusCode::SERVICE_UNAVAILABLE,
            "attachment_storage_unavailable",
            "첨부 저장소가 설정되지 않았습니다.",
        ))
    })
}

fn attachment_storage_failure(headers: &HeaderMap, error: StorageError) -> Response {
    Metrics::global().record_storage_failure();
    match error {
        StorageError::Unavailable(message) => {
            tracing::warn!(reason = %message, "attachment storage is unavailable");
            failure(
                headers,
                StatusCode::SERVICE_UNAVAILABLE,
                "attachment_storage_unavailable",
                "첨부 저장소를 사용할 수 없습니다.",
            )
        }
        StorageError::Failed(message) => {
            tracing::warn!(reason = %message, "attachment storage failed");
            failure(
                headers,
                StatusCode::BAD_GATEWAY,
                "attachment_storage_failed",
                "첨부 저장소가 요청을 처리하지 못했습니다.",
            )
        }
    }
}

fn attachment_rejection(rejection: AttachmentRejection) -> (&'static str, &'static str) {
    (rejection.code(), rejection.message())
}

/// `Content-Disposition` for a file name that may be Korean.
fn content_disposition(file_name: &str) -> HeaderValue {
    let mut encoded = String::new();
    for byte in file_name.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(*byte as char);
            }
            other => encoded.push_str(&format!("%{other:02X}")),
        }
    }
    HeaderValue::from_str(&format!("attachment; filename*=UTF-8''{encoded}"))
        .unwrap_or_else(|_| HeaderValue::from_static("attachment"))
}

/// Finds one attachment of this school and decides what the caller may do.
async fn attachment_for_call(
    state: &AppState,
    headers: &HeaderMap,
    user: &UserRecord,
    capability: Capability,
    attachment_id: &str,
) -> Result<(Uuid, MembershipRole, AttachmentRecord), Box<Response>> {
    let (tenant_id, role) = authorize(state, headers, user, capability).await?;
    let Ok(attachment_id) = Uuid::parse_str(attachment_id) else {
        return Err(Box::new(failure(
            headers,
            StatusCode::BAD_REQUEST,
            "invalid_id",
            "the attachment identifier is not valid",
        )));
    };
    let record = school_collect_db::get_attachment(&state.pool, tenant_id, attachment_id)
        .await
        .map_err(|_| Box::new(storage_failure(headers)))?
        .ok_or_else(|| {
            Box::new(failure(
                headers,
                StatusCode::NOT_FOUND,
                "attachment_not_found",
                "이 첨부 파일을 찾지 못했습니다.",
            ))
        })?;
    Ok((tenant_id, role, record))
}

#[utoipa::path(
    post,
    path = "/v1/collects/{collect_id}/attachments",
    params(("collect_id" = String, Path, description = "Collect identifier")),
    request_body = CreateAttachmentRequest,
    responses(
        (status = 201, description = "Attachment slot opened", body = CreateAttachmentResponse),
        (status = 400, description = "The file violates the attachment policy", body = ApiError),
        (status = 503, description = "Attachment storage is not configured", body = ApiError)
    )
)]
async fn create_attachment(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    Path(collect_id): Path<String>,
    headers: HeaderMap,
    Json(body): Json<CreateAttachmentRequest>,
) -> Response {
    // Storage is configuration: without it this endpoint refuses before any
    // other work, so a misconfigured deployment is obvious and cheap.
    if let Err(response) = attachment_storage(&state, &headers) {
        return *response;
    }
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

    let file_name = match attachments::validate_new_attachment(
        &body.file_name,
        &body.content_type,
        body.byte_size,
    ) {
        Ok(file_name) => file_name,
        Err(rejection) => {
            let (code, message) = attachment_rejection(rejection);
            return failure(&headers, StatusCode::BAD_REQUEST, code, message);
        }
    };
    if !is_valid_item_key(&body.item_key) {
        return failure(
            &headers,
            StatusCode::BAD_REQUEST,
            "invalid_item_key",
            "the item key is not valid",
        );
    }

    let id = Uuid::now_v7();
    let now = Utc::now();
    let attachment = NewAttachment {
        id,
        item_key: body.item_key.clone(),
        file_name,
        content_type: body.content_type.trim().to_ascii_lowercase(),
        byte_size: body.byte_size,
        object_key: attachments::object_key(&tenant_id.to_string(), &id.to_string()),
        expires_at: attachments::expires_at(now),
    };

    match school_collect_db::create_attachment(
        &state.pool,
        tenant_id,
        collect_id,
        user.id,
        attachment,
    )
    .await
    {
        Ok(CreateAttachmentOutcome::Created(record)) => (
            StatusCode::CREATED,
            Json(CreateAttachmentResponse {
                attachment: attachment_dto(&record),
                upload: AttachmentUploadTarget {
                    kind: "api".to_owned(),
                    url: format!("/v1/attachments/{}/content", record.id),
                    method: "PUT".to_owned(),
                },
            }),
        )
            .into_response(),
        Ok(CreateAttachmentOutcome::CollectNotOpen { status }) => failure(
            &headers,
            StatusCode::CONFLICT,
            "collect_not_open",
            format!("a collect in state {status} does not accept attachments"),
        ),
        Ok(CreateAttachmentOutcome::ItemMissing) => failure(
            &headers,
            StatusCode::BAD_REQUEST,
            "item_missing",
            "this collect has no such item",
        ),
        Ok(CreateAttachmentOutcome::NotAssigned) => failure(
            &headers,
            StatusCode::FORBIDDEN,
            "not_assigned",
            "this collect does not target you, so there is nothing to attach",
        ),
        Ok(CreateAttachmentOutcome::AlreadySubmitted) => failure(
            &headers,
            StatusCode::CONFLICT,
            "already_submitted",
            "이미 제출한 답변에는 파일을 더할 수 없습니다.",
        ),
        Ok(CreateAttachmentOutcome::LimitReached) => failure(
            &headers,
            StatusCode::CONFLICT,
            "attachment_limit_reached",
            "첨부할 수 있는 파일 수를 넘었습니다.",
        ),
        Err(_) => storage_failure(&headers),
    }
}

#[utoipa::path(
    get,
    path = "/v1/collects/{collect_id}/attachments",
    params(("collect_id" = String, Path, description = "Collect identifier")),
    responses((status = 200, description = "Attachments of one collect", body = AttachmentListResponse))
)]
async fn list_collect_attachments(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    Path(collect_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (tenant_id, role) = match authorize(&state, &headers, &user, Capability::View).await {
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

    // A manager sees the whole collect; everyone else sees only their own files.
    let records = if role.can_manage_collects() {
        school_collect_db::list_collect_attachments(&state.pool, tenant_id, collect_id).await
    } else {
        school_collect_db::list_submission_attachments(&state.pool, tenant_id, collect_id, user.id)
            .await
    };
    match records {
        Ok(records) => (
            StatusCode::OK,
            Json(AttachmentListResponse {
                attachments: records.iter().map(attachment_dto).collect(),
            }),
        )
            .into_response(),
        Err(_) => storage_failure(&headers),
    }
}

#[utoipa::path(
    get,
    path = "/v1/attachments/{attachment_id}",
    params(("attachment_id" = String, Path, description = "Attachment identifier")),
    responses((status = 200, description = "Attachment metadata", body = AttachmentDto))
)]
async fn attachment_metadata(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    Path(attachment_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (_, role, record) = match attachment_for_call(
        &state,
        &headers,
        &user,
        Capability::View,
        &attachment_id,
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if !attachments::can_read(role, record.user_id == user.id) {
        return failure(
            &headers,
            StatusCode::FORBIDDEN,
            "forbidden",
            "your role does not allow this attachment",
        );
    }
    (StatusCode::OK, Json(attachment_dto(&record))).into_response()
}

#[utoipa::path(
    get,
    path = "/v1/attachments/{attachment_id}/content",
    params(("attachment_id" = String, Path, description = "Attachment identifier")),
    responses((status = 200, description = "Attachment bytes"))
)]
async fn download_attachment(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    Path(attachment_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let storage = match attachment_storage(&state, &headers) {
        Ok(storage) => storage,
        Err(response) => return *response,
    };
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (_, role, record) = match attachment_for_call(
        &state,
        &headers,
        &user,
        Capability::View,
        &attachment_id,
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if !attachments::can_read(role, record.user_id == user.id) {
        return failure(
            &headers,
            StatusCode::FORBIDDEN,
            "forbidden",
            "your role does not allow this attachment",
        );
    }
    let bytes = match storage.get(&record.object_key).await {
        Ok(Some(bytes)) => bytes,
        Ok(None) => {
            // Metadata outlived the object: report it instead of sending nothing.
            return failure(
                &headers,
                StatusCode::NOT_FOUND,
                "attachment_content_missing",
                "첨부 파일의 내용을 찾지 못했습니다.",
            );
        }
        Err(error) => return attachment_storage_failure(&headers, error),
    };

    (
        StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                HeaderValue::from_str(&record.content_type)
                    .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                content_disposition(&record.file_name),
            ),
            (
                axum::http::header::CACHE_CONTROL,
                HeaderValue::from_static("private, no-store"),
            ),
        ],
        bytes,
    )
        .into_response()
}

// The body is raw bytes, which OpenAPI cannot describe as JSON, so this handler
// stays out of the generated document (the slot endpoint carries the contract).
async fn upload_attachment_content(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    Path(attachment_id): Path<String>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let storage = match attachment_storage(&state, &headers) {
        Ok(storage) => storage,
        Err(response) => return *response,
    };
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (tenant_id, _, record) = match attachment_for_call(
        &state,
        &headers,
        &user,
        Capability::Submit,
        &attachment_id,
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return *response,
    };
    if record.user_id != user.id {
        return failure(
            &headers,
            StatusCode::FORBIDDEN,
            "forbidden",
            "남의 첨부 파일에는 내용을 올릴 수 없습니다.",
        );
    }

    let declared = headers
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .split(';')
                .next()
                .unwrap_or(value)
                .trim()
                .to_ascii_lowercase()
        })
        .unwrap_or_default();
    if declared != record.content_type {
        return failure(
            &headers,
            StatusCode::BAD_REQUEST,
            "attachment_type_mismatch",
            "올리려는 파일 형식이 신고한 형식과 다릅니다.",
        );
    }

    let bytes = body.to_vec();
    let checksum = hex::encode(Sha256::digest(&bytes));
    if let Err(error) = storage
        .put(&record.object_key, bytes.clone(), &record.content_type)
        .await
    {
        return attachment_storage_failure(&headers, error);
    }

    match school_collect_db::complete_attachment(
        &state.pool,
        tenant_id,
        record.id,
        bytes.len() as i64,
        &checksum,
        Utc::now(),
    )
    .await
    {
        Ok(CompleteAttachmentOutcome::Stored(stored)) => {
            (StatusCode::OK, Json(attachment_dto(&stored))).into_response()
        }
        Ok(CompleteAttachmentOutcome::AlreadyStored(stored)) => {
            (StatusCode::OK, Json(attachment_dto(&stored))).into_response()
        }
        Ok(CompleteAttachmentOutcome::NotFound) => failure(
            &headers,
            StatusCode::NOT_FOUND,
            "attachment_not_found",
            "이 첨부 파일을 찾지 못했습니다.",
        ),
        Ok(CompleteAttachmentOutcome::Expired) => failure(
            &headers,
            StatusCode::CONFLICT,
            "attachment_expired",
            "보존 기간이 지나 이 첨부 파일은 더 받을 수 없습니다.",
        ),
        Ok(CompleteAttachmentOutcome::SizeMismatch { declared, received }) => {
            // The bytes are not what the slot promised, so drop them again.
            let _ = storage.delete(&record.object_key).await;
            failure(
                &headers,
                StatusCode::BAD_REQUEST,
                "attachment_size_mismatch",
                format!("신고한 크기({declared})와 실제 크기({received})가 다릅니다."),
            )
        }
        Err(_) => storage_failure(&headers),
    }
}

#[utoipa::path(
    delete,
    path = "/v1/attachments/{attachment_id}",
    params(("attachment_id" = String, Path, description = "Attachment identifier")),
    responses((status = 200, description = "Attachment removed", body = ApiError))
)]
async fn delete_attachment(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    Path(attachment_id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let storage = match attachment_storage(&state, &headers) {
        Ok(storage) => storage,
        Err(response) => return *response,
    };
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (tenant_id, role, record) = match attachment_for_call(
        &state,
        &headers,
        &user,
        Capability::View,
        &attachment_id,
    )
    .await
    {
        Ok(value) => value,
        Err(response) => return *response,
    };

    // An answer that was handed in keeps its files unless a manager says otherwise.
    let submitted = !role.can_manage_collects()
        && matches!(
            school_collect_db::get_submission(&state.pool, tenant_id, record.collect_id, record.user_id)
                .await,
            Ok(Some(submission)) if submission.status == "submitted"
        );
    if !attachments::can_delete(role, record.user_id == user.id, submitted) {
        return failure(
            &headers,
            StatusCode::FORBIDDEN,
            "forbidden",
            "제출한 답변의 첨부 파일은 관리자만 지울 수 있습니다.",
        );
    }

    if let Err(error) = storage.delete(&record.object_key).await {
        return attachment_storage_failure(&headers, error);
    }
    match school_collect_db::delete_attachment(
        &state.pool,
        tenant_id,
        record.id,
        user.id,
        Utc::now(),
    )
    .await
    {
        Ok(DeleteAttachmentOutcome::Deleted { .. }) => StatusCode::NO_CONTENT.into_response(),
        Ok(DeleteAttachmentOutcome::NotFound) => failure(
            &headers,
            StatusCode::NOT_FOUND,
            "attachment_not_found",
            "이 첨부 파일을 찾지 못했습니다.",
        ),
        Err(_) => storage_failure(&headers),
    }
}

fn storage_failure(headers: &HeaderMap) -> Response {
    Metrics::global().record_storage_failure();
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

/// Item keys are part of the submission payload contract, so they are checked
/// here instead of being reported as a database error.
fn is_valid_item_key(value: &str) -> bool {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first.is_ascii_lowercase() => {}
        _ => return false,
    }
    chars.all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
    })
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

/// Operational counters in the Prometheus text format.
///
/// Like `/health` and `/ready` this exposes no tenant data: only request,
/// rejection, and storage-failure counts.
async fn metrics(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut body = Metrics::global().render();
    // Delivery backlog is a database fact; a database that is merely down must
    // not turn the metrics endpoint into an error.
    if let Ok(stats) = school_collect_db::outbox_stats(&state.pool).await {
        for (name, help, value) in [
            (
                "school_collect_outbox_pending",
                "Outbox events waiting for delivery",
                stats.pending,
            ),
            (
                "school_collect_outbox_published_total",
                "Outbox events the broker acknowledged",
                stats.published,
            ),
            (
                "school_collect_outbox_dead_lettered",
                "Outbox events that exhausted their delivery attempts",
                stats.dead_lettered,
            ),
        ] {
            body.push_str(&format!("# HELP {name} {help}\n"));
            body.push_str(&format!("# TYPE {name} gauge\n"));
            body.push_str(&format!("{name} {value}\n"));
        }
    }
    (
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
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

/// Invitations stay valid for two weeks. This slice has no email provider, so
/// the inviter shares the one-time code out of band.
const INVITATION_VALID_DAYS: i64 = 14;

fn normalize_email(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.chars().count() < 3 || trimmed.chars().count() > 254 {
        return None;
    }
    if trimmed
        .chars()
        .any(|character| character.is_whitespace() || character.is_control())
    {
        return None;
    }
    let mut parts = trimmed.split('@');
    let local = parts.next().unwrap_or("");
    let domain = parts.next().unwrap_or("");
    if local.is_empty() || domain.is_empty() || parts.next().is_some() {
        return None;
    }
    if local.starts_with('.') || local.ends_with('.') || local.contains("..") {
        return None;
    }
    if !domain.contains('.') || domain.starts_with('.') || domain.ends_with('.') {
        return None;
    }
    Some(trimmed.to_lowercase())
}

/// Admin rights are never handed out through an invitation.
fn invitation_role(value: Option<&str>) -> Option<&'static str> {
    let requested = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("contributor");
    match requested {
        "contributor" => Some("contributor"),
        "coordinator" => Some("coordinator"),
        "viewer" => Some("viewer"),
        _ => None,
    }
}

fn generate_invitation_code() -> String {
    let mut bytes = [0u8; 32];
    bytes[..16].copy_from_slice(Uuid::new_v4().as_bytes());
    bytes[16..].copy_from_slice(Uuid::new_v4().as_bytes());
    hex::encode(bytes)
}

fn hash_invitation_code(code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(code.as_bytes());
    hex::encode(hasher.finalize())
}

fn invitation_dto(record: &InvitationRecord) -> InvitationDto {
    InvitationDto {
        id: record.id.to_string(),
        email: record.email.clone(),
        role: record.role.clone(),
        status: record.status.clone(),
        created_at: timestamp(record.created_at),
        expires_at: timestamp(record.expires_at),
        accepted_at: record.accepted_at.map(timestamp),
    }
}

#[utoipa::path(
    post,
    path = "/v1/invitations",
    request_body = CreateInvitationRequest,
    responses(
        (status = 201, description = "Invitation created; the code is returned once", body = CreateInvitationResponse),
        (status = 409, description = "A pending invitation for this address already exists", body = ApiError)
    )
)]
async fn create_invitation(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
    Json(body): Json<CreateInvitationRequest>,
) -> Response {
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (tenant_id, _) = match authorize(&state, &headers, &user, Capability::Manage).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let Some(email) = normalize_email(&body.email) else {
        return failure(
            &headers,
            StatusCode::BAD_REQUEST,
            "invalid_email",
            "a valid email address is required",
        );
    };
    let Some(role) = invitation_role(body.role.as_deref()) else {
        return failure(
            &headers,
            StatusCode::BAD_REQUEST,
            "invalid_role",
            "the role must be contributor, coordinator, or viewer",
        );
    };

    let code = generate_invitation_code();
    let expires_at = Utc::now() + Duration::days(INVITATION_VALID_DAYS);
    match school_collect_db::create_invitation(
        &state.pool,
        tenant_id,
        user.id,
        &email,
        role,
        &hash_invitation_code(&code),
        expires_at,
    )
    .await
    {
        Ok(CreateInvitationOutcome::Created(record)) => (
            StatusCode::CREATED,
            Json(CreateInvitationResponse {
                invitation: invitation_dto(&record),
                code,
            }),
        )
            .into_response(),
        Ok(CreateInvitationOutcome::Duplicate) => failure(
            &headers,
            StatusCode::CONFLICT,
            "invitation_exists",
            "a pending invitation for this address already exists",
        ),
        Err(_) => storage_failure(&headers),
    }
}

#[utoipa::path(
    get,
    path = "/v1/invitations",
    responses((status = 200, description = "Invitations of the caller's school", body = InvitationListResponse))
)]
async fn list_invitations(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
) -> Response {
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (tenant_id, _) = match authorize(&state, &headers, &user, Capability::Manage).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match school_collect_db::list_invitations(&state.pool, tenant_id).await {
        Ok(rows) => Json(InvitationListResponse {
            invitations: rows.iter().map(invitation_dto).collect(),
        })
        .into_response(),
        Err(_) => storage_failure(&headers),
    }
}

#[utoipa::path(
    post,
    path = "/v1/invitations/{invitation_id}/revoke",
    params(("invitation_id" = String, Path, description = "Invitation identifier")),
    responses(
        (status = 200, description = "Invitation revoked", body = InvitationDto),
        (status = 404, description = "Pending invitation not found", body = ApiError)
    )
)]
async fn revoke_invitation(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
    Path(invitation_id): Path<String>,
) -> Response {
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (tenant_id, _) = match authorize(&state, &headers, &user, Capability::Manage).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    let Ok(invitation_id) = Uuid::parse_str(invitation_id.trim()) else {
        return failure(
            &headers,
            StatusCode::BAD_REQUEST,
            "invitation_invalid",
            "the invitation identifier is not valid",
        );
    };
    match school_collect_db::revoke_invitation(&state.pool, tenant_id, invitation_id).await {
        Ok(Some(record)) => Json(invitation_dto(&record)).into_response(),
        Ok(None) => failure(
            &headers,
            StatusCode::NOT_FOUND,
            "invitation_not_found",
            "no pending invitation with this identifier",
        ),
        Err(_) => storage_failure(&headers),
    }
}

#[utoipa::path(
    post,
    path = "/v1/invitations/accept",
    request_body = AcceptInvitationRequest,
    responses(
        (status = 200, description = "Invitation accepted; the membership now exists", body = AcceptInvitationResponse),
        (status = 403, description = "The invitation belongs to another address", body = ApiError),
        (status = 410, description = "The invitation expired", body = ApiError)
    )
)]
async fn accept_invitation(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
    Json(body): Json<AcceptInvitationRequest>,
) -> Response {
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let code = body.code.trim().to_lowercase();
    if code.len() != 64 || !code.chars().all(|character| character.is_ascii_hexdigit()) {
        return failure(
            &headers,
            StatusCode::BAD_REQUEST,
            "invalid_code",
            "the invitation code is not valid",
        );
    }
    // The address comes from the verified token, never from the request body.
    let Some(email) = principal
        .email
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase)
    else {
        return failure(
            &headers,
            StatusCode::BAD_REQUEST,
            "email_required",
            "your verified email address is required to accept an invitation",
        );
    };

    match school_collect_db::accept_invitation(
        &state.pool,
        &hash_invitation_code(&code),
        &email,
        user.id,
    )
    .await
    {
        Ok(AcceptInvitationOutcome::Accepted { tenant_id, role }) => {
            let memberships = match school_collect_db::list_memberships(&state.pool, user.id).await
            {
                Ok(rows) => rows,
                Err(_) => return storage_failure(&headers),
            };
            match memberships
                .into_iter()
                .find(|row| row.tenant_id == tenant_id)
            {
                Some(row) => Json(AcceptInvitationResponse {
                    membership: MembershipDto {
                        tenant_id: tenant_id.to_string(),
                        tenant_name: row.tenant_name,
                        role,
                    },
                })
                .into_response(),
                None => storage_failure(&headers),
            }
        }
        Ok(AcceptInvitationOutcome::NotFound) => failure(
            &headers,
            StatusCode::NOT_FOUND,
            "invitation_not_found",
            "the invitation code is not valid",
        ),
        Ok(AcceptInvitationOutcome::Expired) => failure(
            &headers,
            StatusCode::GONE,
            "invitation_expired",
            "this invitation has expired",
        ),
        Ok(AcceptInvitationOutcome::EmailMismatch) => failure(
            &headers,
            StatusCode::FORBIDDEN,
            "invitation_email_mismatch",
            "this invitation was issued to a different email address",
        ),
        Ok(AcceptInvitationOutcome::AlreadyUsed { status }) => failure(
            &headers,
            StatusCode::CONFLICT,
            "invitation_used",
            format!("this invitation is already {status}"),
        ),
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

    let items = match collect_items_from(&headers, &body.items) {
        Ok(items) => items,
        Err(response) => return response,
    };

    let mut assignees: Vec<Uuid> = Vec::with_capacity(body.assignee_user_ids.len());
    for raw in &body.assignee_user_ids {
        match Uuid::parse_str(raw) {
            Ok(id) => assignees.push(id),
            Err(_) => {
                return failure(
                    &headers,
                    StatusCode::BAD_REQUEST,
                    "invalid_assignee",
                    "assigneeUserIds must contain user identifiers",
                );
            }
        }
    }
    let assignee_ids = (!body.assignee_user_ids.is_empty()).then_some(assignees.as_slice());

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
        &school_collect_db::NewCollect {
            title,
            description: body.description.trim(),
            due_at,
            items: &items,
            assignee_ids,
        },
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

    collect_detail_response(&state, &headers, tenant_id, collect_id, user.id).await
}

/// Builds the collect detail payload for an already authorized caller.
async fn collect_detail_response(
    state: &AppState,
    headers: &HeaderMap,
    tenant_id: Uuid,
    collect_id: Uuid,
    user_id: Uuid,
) -> Response {
    let record = match school_collect_db::get_collect(&state.pool, tenant_id, collect_id).await {
        Ok(Some(record)) => record,
        Ok(None) => {
            return failure(
                headers,
                StatusCode::NOT_FOUND,
                "not_found",
                "the collect was not found in this school",
            );
        }
        Err(_) => return storage_failure(headers),
    };

    let submission = match school_collect_db::get_submission(
        &state.pool,
        tenant_id,
        collect_id,
        user_id,
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
        Err(_) => return storage_failure(headers),
    };

    let items =
        match school_collect_db::list_collect_items(&state.pool, tenant_id, collect_id).await {
            Ok(rows) => rows
                .into_iter()
                .map(|row| CollectItemDto {
                    key: row.item_key,
                    label: row.label,
                    required: row.required,
                    position: row.position,
                })
                .collect(),
            Err(_) => return storage_failure(headers),
        };

    let progress =
        match school_collect_db::collect_progress(&state.pool, tenant_id, collect_id).await {
            Ok(value) => value,
            Err(_) => return storage_failure(headers),
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
        items,
        progress: CollectProgressDto {
            assigned: progress.assigned,
            submitted: progress.submitted,
        },
    })
    .into_response()
}

/// One CSV field. Quotes, commas, and line breaks force quoting; a quote inside
/// a value is doubled so a member's own text cannot break the columns.
fn csv_field(value: &str) -> String {
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

/// Result export of one collect.
///
/// Assigned members appear whether or not they submitted, so a missing answer
/// is visible in the file instead of silently dropping the row.
fn render_collect_csv(
    items: &[school_collect_db::CollectItemRecord],
    rows: &[school_collect_db::CollectStatusRowRecord],
    submissions: &std::collections::HashMap<Uuid, serde_json::Value>,
) -> String {
    let mut header: Vec<String> = vec![
        "이름".to_owned(),
        "역할".to_owned(),
        "배정 상태".to_owned(),
        "제출 상태".to_owned(),
        "제출 시각".to_owned(),
    ];
    header.extend(items.iter().map(|item| item.label.clone()));

    let mut output = String::from("\u{feff}");
    output.push_str(
        &header
            .iter()
            .map(|value| csv_field(value))
            .collect::<Vec<_>>()
            .join(","),
    );
    output.push_str("\r\n");

    for row in rows {
        let mut fields: Vec<String> = vec![
            csv_field(row.display_name.as_deref().unwrap_or("")),
            csv_field(&row.role),
            csv_field(row.assignment_status.as_deref().unwrap_or("")),
            csv_field(row.submission_status.as_deref().unwrap_or("")),
            csv_field(&row.submitted_at.map(timestamp).unwrap_or_default()),
        ];
        let payload = submissions.get(&row.user_id);
        for item in items {
            let value = payload
                .and_then(|payload| payload.get(&item.item_key))
                .and_then(|value| value.as_str())
                .unwrap_or("");
            fields.push(csv_field(value));
        }
        output.push_str(&fields.join(","));
        output.push_str("\r\n");
    }

    output
}

#[utoipa::path(
    get,
    path = "/v1/collects/{collect_id}/export",
    params(("collect_id" = String, Path, description = "Collect identifier")),
    responses((status = 200, description = "Collect results as CSV"))
)]
async fn export_collect(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
    Path(collect_id): Path<String>,
) -> Response {
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (tenant_id, _) = match authorize(&state, &headers, &user, Capability::Manage).await {
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
    match school_collect_db::get_collect(&state.pool, tenant_id, collect_id).await {
        Ok(Some(_)) => {}
        Ok(None) => {
            return failure(
                &headers,
                StatusCode::NOT_FOUND,
                "not_found",
                "the collect was not found in this school",
            );
        }
        Err(_) => return storage_failure(&headers),
    }

    let items =
        match school_collect_db::list_collect_items(&state.pool, tenant_id, collect_id).await {
            Ok(items) => items,
            Err(_) => return storage_failure(&headers),
        };
    let rows =
        match school_collect_db::list_collect_status(&state.pool, tenant_id, collect_id).await {
            Ok(rows) => rows,
            Err(_) => return storage_failure(&headers),
        };
    let submissions =
        match school_collect_db::list_collect_submissions(&state.pool, tenant_id, collect_id).await
        {
            Ok(submissions) => submissions,
            Err(_) => return storage_failure(&headers),
        };
    let payloads = submissions
        .into_iter()
        .map(|submission| (submission.user_id, submission.payload))
        .collect();

    // Exporting results is an access to member work, so it is recorded.
    if school_collect_db::record_audit_event(
        &state.pool,
        tenant_id,
        Some(user.id),
        "collect.exported",
        "collect",
        Some(collect_id),
    )
    .await
    .is_err()
    {
        return storage_failure(&headers);
    }

    let body = render_collect_csv(&items, &rows, &payloads);
    (
        StatusCode::OK,
        [
            (
                axum::http::header::CONTENT_TYPE,
                "text/csv; charset=utf-8".to_owned(),
            ),
            (
                axum::http::header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"collect-{collect_id}.csv\""),
            ),
            (
                axum::http::HeaderName::from_static("x-request-id"),
                request_id_of(&headers),
            ),
        ],
        body,
    )
        .into_response()
}

#[utoipa::path(
    get,
    path = "/v1/collects/{collect_id}/status",
    params(("collect_id" = String, Path, description = "Collect identifier")),
    responses((status = 200, description = "Submission status per assigned member", body = CollectStatusResponse))
)]
async fn collect_status(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
    Path(collect_id): Path<String>,
) -> Response {
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (tenant_id, _) = match authorize(&state, &headers, &user, Capability::Manage).await {
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
    match school_collect_db::get_collect(&state.pool, tenant_id, collect_id).await {
        Ok(Some(_)) => {}
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

    let rows =
        match school_collect_db::list_collect_status(&state.pool, tenant_id, collect_id).await {
            Ok(rows) => rows,
            Err(_) => return storage_failure(&headers),
        };
    let progress =
        match school_collect_db::collect_progress(&state.pool, tenant_id, collect_id).await {
            Ok(value) => value,
            Err(_) => return storage_failure(&headers),
        };

    Json(CollectStatusResponse {
        assigned: progress.assigned,
        submitted: progress.submitted,
        rows: rows
            .into_iter()
            .map(|row| CollectStatusRowDto {
                user_id: row.user_id.to_string(),
                display_name: row.display_name,
                role: row.role,
                assignment_status: row.assignment_status,
                submission_status: row.submission_status,
                submitted_at: row.submitted_at.map(timestamp),
            })
            .collect(),
    })
    .into_response()
}

#[utoipa::path(
    get,
    path = "/v1/members",
    responses((status = 200, description = "Members of the active school", body = MemberListResponse))
)]
async fn list_members(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
) -> Response {
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (tenant_id, _) = match authorize(&state, &headers, &user, Capability::Manage).await {
        Ok(value) => value,
        Err(response) => return *response,
    };
    match school_collect_db::list_members(&state.pool, tenant_id).await {
        Ok(rows) => Json(MemberListResponse {
            members: rows
                .into_iter()
                .map(|row| MemberDto {
                    user_id: row.user_id.to_string(),
                    display_name: row.display_name,
                    role: row.role,
                })
                .collect(),
        })
        .into_response(),
        Err(_) => storage_failure(&headers),
    }
}

#[utoipa::path(
    get,
    path = "/v1/assignments",
    responses((status = 200, description = "Collects this caller must submit", body = AssignmentListResponse))
)]
async fn list_assignments(
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
    match school_collect_db::list_assignments(&state.pool, tenant_id, user.id).await {
        Ok(rows) => Json(AssignmentListResponse {
            assignments: rows
                .into_iter()
                .map(|row| AssignmentDto {
                    collect_id: row.collect_id.to_string(),
                    title: row.title,
                    status: row.status,
                    due_at: row.due_at.map(timestamp),
                    assignment_status: row.assignment_status,
                    submission_status: row.submission_status,
                    submission_version: row.submission_version,
                })
                .collect(),
        })
        .into_response(),
        Err(_) => storage_failure(&headers),
    }
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

/// Shared validation for the item list of a collect.
#[allow(clippy::result_large_err)] // the error is the HTTP response we return
fn collect_items_from(
    headers: &HeaderMap,
    input: &[school_collect_contracts::CollectItemInput],
) -> Result<Vec<NewCollectItem>, Response> {
    if input.len() > 50 {
        return Err(failure(
            headers,
            StatusCode::BAD_REQUEST,
            "too_many_items",
            "a collect can define at most 50 items",
        ));
    }

    let mut items: Vec<NewCollectItem> = Vec::with_capacity(input.len());
    for item in input {
        let key = item.key.trim();
        if !is_valid_item_key(key) {
            return Err(failure(
                headers,
                StatusCode::BAD_REQUEST,
                "invalid_item_key",
                "item keys must match ^[a-z][a-z0-9_]*$",
            ));
        }
        let label = item.label.trim();
        if label.is_empty() || label.chars().count() > 120 {
            return Err(failure(
                headers,
                StatusCode::BAD_REQUEST,
                "invalid_item_label",
                "item labels must be between 1 and 120 characters",
            ));
        }
        if items.iter().any(|existing| existing.key == key) {
            return Err(failure(
                headers,
                StatusCode::BAD_REQUEST,
                "duplicate_item_key",
                "item keys must be unique within a collect",
            ));
        }
        items.push(NewCollectItem {
            key: key.to_owned(),
            label: label.to_owned(),
            required: item.required,
        });
    }

    Ok(items)
}

#[utoipa::path(
    put,
    path = "/v1/collects/{collect_id}/items",
    params(("collect_id" = String, Path, description = "Collect identifier")),
    responses(
        (status = 200, description = "Items updated", body = CollectDetailResponse),
        (status = 409, description = "Version conflict or an answered item cannot be removed")
    )
)]
async fn update_collect_items(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
    Path(collect_id): Path<String>,
    Json(body): Json<UpdateCollectItemsRequest>,
) -> Response {
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (tenant_id, _) = match authorize(&state, &headers, &user, Capability::Manage).await {
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
    let items = match collect_items_from(&headers, &body.items) {
        Ok(items) => items,
        Err(response) => return response,
    };

    match school_collect_db::update_collect_items(
        &state.pool,
        tenant_id,
        collect_id,
        user.id,
        body.expected_version,
        &items,
    )
    .await
    {
        Ok(UpdateItemsOutcome::Updated(_)) => {
            collect_detail_response(&state, &headers, tenant_id, collect_id, user.id).await
        }
        Ok(UpdateItemsOutcome::VersionConflict { current_version }) => {
            conflict_response(&headers, current_version)
        }
        Ok(UpdateItemsOutcome::RemovalBlocked { key }) => failure(
            &headers,
            StatusCode::CONFLICT,
            "item_has_answers",
            format!("item {key} already has answers and cannot be removed"),
        ),
        Ok(UpdateItemsOutcome::NotEditable { status }) => failure(
            &headers,
            StatusCode::CONFLICT,
            "collect_not_editable",
            format!("a collect in state {status} cannot be edited"),
        ),
        Ok(UpdateItemsOutcome::NotFound) => failure(
            &headers,
            StatusCode::NOT_FOUND,
            "not_found",
            "the collect was not found in this school",
        ),
        Err(_) => storage_failure(&headers),
    }
}

#[utoipa::path(
    put,
    path = "/v1/collects/{collect_id}/assignments",
    params(("collect_id" = String, Path, description = "Collect identifier")),
    responses(
        (status = 200, description = "Targets updated", body = CollectDetailResponse),
        (status = 409, description = "Version conflict or an answered target cannot be removed")
    )
)]
async fn update_collect_assignments(
    State(state): State<Arc<AppState>>,
    Extension(principal): Extension<VerifiedPrincipal>,
    headers: HeaderMap,
    Path(collect_id): Path<String>,
    Json(body): Json<UpdateCollectAssignmentsRequest>,
) -> Response {
    let user = match current_user(&state, &principal, &headers).await {
        Ok(user) => user,
        Err(response) => return *response,
    };
    let (tenant_id, _) = match authorize(&state, &headers, &user, Capability::Manage).await {
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
    if body.assignee_user_ids.len() > 200 {
        return failure(
            &headers,
            StatusCode::BAD_REQUEST,
            "too_many_assignees",
            "a collect can target at most 200 members",
        );
    }

    let mut assignees = Vec::with_capacity(body.assignee_user_ids.len());
    for raw in &body.assignee_user_ids {
        let Ok(user_id) = Uuid::parse_str(raw.trim()) else {
            return failure(
                &headers,
                StatusCode::BAD_REQUEST,
                "invalid_assignee",
                "assignee ids must be UUIDs",
            );
        };
        if assignees.contains(&user_id) {
            continue;
        }
        assignees.push(user_id);
    }

    match school_collect_db::update_collect_assignments(
        &state.pool,
        tenant_id,
        collect_id,
        user.id,
        body.expected_version,
        &assignees,
    )
    .await
    {
        Ok(UpdateAssignmentsOutcome::Updated(_)) => {
            collect_detail_response(&state, &headers, tenant_id, collect_id, user.id).await
        }
        Ok(UpdateAssignmentsOutcome::VersionConflict { current_version }) => {
            conflict_response(&headers, current_version)
        }
        Ok(UpdateAssignmentsOutcome::NotAssignable { user_id }) => failure(
            &headers,
            StatusCode::CONFLICT,
            "target_not_assignable",
            format!("{user_id} is not a member who can submit in this school"),
        ),
        Ok(UpdateAssignmentsOutcome::RemovalBlocked { user_id }) => failure(
            &headers,
            StatusCode::CONFLICT,
            "target_has_answers",
            format!("{user_id} already saved work and cannot be removed"),
        ),
        Ok(UpdateAssignmentsOutcome::NotEditable { status }) => failure(
            &headers,
            StatusCode::CONFLICT,
            "collect_not_editable",
            format!("a collect in state {status} cannot be edited"),
        ),
        Ok(UpdateAssignmentsOutcome::NotFound) => failure(
            &headers,
            StatusCode::NOT_FOUND,
            "not_found",
            "the collect was not found in this school",
        ),
        Err(_) => storage_failure(&headers),
    }
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
        Ok(SaveDraftOutcome::NotAssigned) => failure(
            &headers,
            StatusCode::FORBIDDEN,
            "not_assigned",
            "this collect does not target you, so there is nothing to fill in",
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
        Ok(SubmitOutcome::NotAssigned) => failure(
            &headers,
            StatusCode::FORBIDDEN,
            "not_assigned",
            "this collect does not target you, so there is nothing to send",
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
            storage: None,
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
    async fn attachment_routes_refuse_without_configured_storage() {
        let response = router(
            unreachable_state(),
            HeaderValue::from_static("http://127.0.0.1:1420"),
            AuthState::development_disabled(),
        )
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/v1/collects/00000000-0000-7000-8000-000000000000/attachments")
                .header("content-type", "application/json")
                .header("x-tenant-id", "00000000-0000-7000-8000-000000000001")
                .body(Body::from(
                    r#"{"itemKey":"plan","fileName":"plan.pdf","contentType":"application/pdf","byteSize":10}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

        // Missing storage is an operator error, so the feature refuses instead of
        // pretending the upload worked.
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .expect("body");
        let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
        assert_eq!(body["code"], "attachment_storage_unavailable");
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

    fn oidc_state() -> AuthState {
        let values = HashMap::from([
            ("OIDC_ISSUER_URL", "https://id.example.test"),
            ("OIDC_AUDIENCE", "authenticated"),
        ]);
        let config =
            OidcConfig::from_env(|key| values.get(key).map(|value| (*value).to_owned())).unwrap();
        AuthState::oidc(OidcVerifier::new(config).unwrap())
    }

    async fn body_json(response: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("body");
        serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)
    }

    #[tokio::test]
    async fn every_rejection_carries_the_request_id_it_answers_to() {
        // Client-supplied id: it must be echoed on the response and in the body.
        let response = router(
            unreachable_state(),
            HeaderValue::from_static("http://127.0.0.1:1420"),
            oidc_state(),
        )
        .oneshot(
            Request::builder()
                .uri("/v1/session")
                .header("x-request-id", "client-request-7")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(
            response
                .headers()
                .get("x-request-id")
                .and_then(|value| value.to_str().ok()),
            Some("client-request-7")
        );
        let body = body_json(response).await;
        assert_eq!(body["code"], "unauthorized");
        assert_eq!(body["requestId"], "client-request-7");
    }

    #[tokio::test]
    async fn generated_request_ids_are_returned_when_the_client_sends_none() {
        let response = router(
            unreachable_state(),
            HeaderValue::from_static("http://127.0.0.1:1420"),
            oidc_state(),
        )
        .oneshot(
            Request::builder()
                .uri("/v1/session")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        let request_id = response
            .headers()
            .get("x-request-id")
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default()
            .to_owned();
        assert!(!request_id.is_empty());
        assert_ne!(request_id, "request-untracked");

        let body = body_json(response).await;
        assert_eq!(body["requestId"], request_id);
    }

    #[tokio::test]
    async fn rejection_bodies_never_repeat_the_presented_token() {
        let token = "Bearer eyJhbGciOiJFUzI1NiJ9.forged.signature";
        let response = router(
            unreachable_state(),
            HeaderValue::from_static("http://127.0.0.1:1420"),
            oidc_state(),
        )
        .oneshot(
            Request::builder()
                .uri("/v1/session")
                .header("authorization", token)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = body_json(response).await;
        let rendered = body.to_string();
        assert_eq!(body["message"], "the access token could not be verified");
        assert!(!rendered.contains("forged.signature"));
        assert!(!rendered.contains("eyJhbGciOiJFUzI1NiJ9"));
        assert!(!rendered.contains("Bearer"));
    }

    #[tokio::test]
    async fn authentication_unavailable_is_reported_with_a_request_id() {
        let response = router(
            unreachable_state(),
            HeaderValue::from_static("http://127.0.0.1:1420"),
            AuthState {
                mode: AuthMode::Oidc,
                verifier: None,
            },
        )
        .oneshot(
            Request::builder()
                .uri("/v1/session")
                .header("x-request-id", "ops-check-1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        let body = body_json(response).await;
        assert_eq!(body["code"], "authentication_unavailable");
        assert_eq!(body["requestId"], "ops-check-1");
    }

    #[tokio::test]
    async fn tenant_header_must_be_a_present_valid_identifier() {
        let missing = HeaderMap::new();
        let response = tenant_id_from(&missing).expect_err("missing tenant");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body_json(*response).await;
        assert_eq!(body["code"], "tenant_required");

        let mut invalid = HeaderMap::new();
        invalid.insert("x-tenant-id", HeaderValue::from_static("not-a-uuid"));
        let response = tenant_id_from(&invalid).expect_err("invalid tenant");
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = body_json(*response).await;
        assert_eq!(body["code"], "tenant_invalid");

        let tenant = Uuid::now_v7();
        let mut valid = HeaderMap::new();
        valid.insert(
            "x-tenant-id",
            HeaderValue::from_str(&tenant.to_string()).unwrap(),
        );
        assert_eq!(tenant_id_from(&valid).unwrap(), tenant);
    }

    #[tokio::test]
    async fn metrics_are_exposed_without_tenant_data() {
        let response = router(
            unreachable_state(),
            HeaderValue::from_static("http://127.0.0.1:1420"),
            AuthState::development_disabled(),
        )
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .expect("body");
        let text = String::from_utf8(bytes.to_vec()).expect("utf8");
        assert!(text.contains("school_collect_http_requests_total"));
        assert!(text.contains("school_collect_auth_failures_total"));
        assert!(text.contains("school_collect_authorization_denials_total"));
    }

    #[test]
    fn csv_fields_quote_only_when_needed() {
        assert_eq!(csv_field("평범한 값"), "평범한 값");
        assert_eq!(csv_field("쉼표, 포함"), "\"쉼표, 포함\"");
        assert_eq!(csv_field("따옴표\" 포함"), "\"따옴표\"\" 포함\"");
        assert_eq!(csv_field("줄\n바꿈"), "\"줄\n바꿈\"");
        assert_eq!(csv_field(""), "");
    }

    #[test]
    fn csv_export_keeps_missing_answers_visible() {
        let items = vec![
            school_collect_db::CollectItemRecord {
                item_key: "title".to_owned(),
                label: "제목".to_owned(),
                required: true,
                position: 0,
            },
            school_collect_db::CollectItemRecord {
                item_key: "note".to_owned(),
                label: "비고".to_owned(),
                required: false,
                position: 1,
            },
        ];
        let rows = vec![
            school_collect_db::CollectStatusRowRecord {
                user_id: Uuid::now_v7(),
                display_name: Some("제출자".to_owned()),
                role: "contributor".to_owned(),
                assignment_status: Some("submitted".to_owned()),
                submission_status: Some("submitted".to_owned()),
                submitted_at: Some(Utc::now()),
            },
            school_collect_db::CollectStatusRowRecord {
                user_id: Uuid::now_v7(),
                display_name: Some("미제출, 담당".to_owned()),
                role: "contributor".to_owned(),
                assignment_status: Some("assigned".to_owned()),
                submission_status: None,
                submitted_at: None,
            },
        ];
        let mut submissions = std::collections::HashMap::new();
        submissions.insert(
            rows[0].user_id,
            serde_json::json!({ "title": "값, 포함", "note": "메모" }),
        );

        let csv = render_collect_csv(&items, &rows, &submissions);
        let lines: Vec<&str> = csv.lines().collect();

        assert!(
            csv.starts_with('\u{feff}'),
            "Excel needs the BOM for Korean"
        );
        assert_eq!(
            lines[0].trim_start_matches('\u{feff}'),
            "이름,역할,배정 상태,제출 상태,제출 시각,제목,비고"
        );
        assert!(
            lines[1].contains("\"값, 포함\""),
            "commas stay inside one field"
        );
        assert!(!lines[1].contains("\"미제출, 담당\""));
        assert!(lines[2].contains("\"미제출, 담당\""));
        assert!(
            lines[2].ends_with(",,"),
            "an unsubmitted member keeps empty item columns"
        );
    }

    #[test]
    fn invitation_emails_are_normalized_or_rejected() {
        assert_eq!(
            normalize_email("  Teacher.Name@Example.School.KR ").as_deref(),
            Some("teacher.name@example.school.kr")
        );
        for invalid in [
            "",
            "not-an-email",
            "no-domain@",
            "@no-local.example",
            "two@@example.com",
            "no-dot@example",
            "spaces in@example.com",
            ".leading@example.com",
            "trailing.@example.com",
        ] {
            assert!(
                normalize_email(invalid).is_none(),
                "{invalid} should not be a valid invited address"
            );
        }
    }

    #[test]
    fn invitation_roles_exclude_admin_and_default_to_contributor() {
        assert_eq!(invitation_role(None), Some("contributor"));
        assert_eq!(invitation_role(Some("  ")), Some("contributor"));
        assert_eq!(invitation_role(Some("contributor")), Some("contributor"));
        assert_eq!(invitation_role(Some("coordinator")), Some("coordinator"));
        assert_eq!(invitation_role(Some("viewer")), Some("viewer"));
        assert_eq!(invitation_role(Some("admin")), None);
        assert_eq!(invitation_role(Some("owner")), None);
    }

    #[test]
    fn invitation_codes_are_random_and_stored_hashed() {
        let first = generate_invitation_code();
        let second = generate_invitation_code();

        assert_eq!(first.len(), 64);
        assert!(first.chars().all(|character| character.is_ascii_hexdigit()));
        assert_ne!(first, second, "two codes must not collide");

        let hash = hash_invitation_code(&first);
        assert_eq!(hash.len(), 64);
        assert_ne!(hash, first, "the stored value must not be the code itself");
        assert_eq!(hash, hash_invitation_code(&first));
        assert_ne!(hash, hash_invitation_code(&second));
    }
}
