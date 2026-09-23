use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ServiceStatus {
    pub status: &'static str,
    pub service: &'static str,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct PrincipalResponse {
    pub issuer: String,
    pub subject: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CollectSummary {
    pub id: String,
    pub title: String,
    pub status: String,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct CreateCollectRequest {
    pub title: String,
    pub description: String,
    pub due_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SaveSubmissionRequest {
    pub expected_version: i64,
    pub payload: serde_json::Value,
}
