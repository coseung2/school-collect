use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ServiceStatus {
    pub status: &'static str,
    pub service: &'static str,
}

/// Every failed request returns this shape so the client never has to parse a
/// prose error or a framework default page.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PrincipalResponse {
    pub issuer: String,
    pub subject: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub id: String,
    pub issuer: String,
    pub subject: String,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MembershipDto {
    pub tenant_id: String,
    pub tenant_name: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SessionResponse {
    pub user: UserDto,
    pub memberships: Vec<MembershipDto>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TenantListResponse {
    pub tenants: Vec<MembershipDto>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTenantRequest {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CollectDto {
    pub id: String,
    pub title: String,
    pub status: String,
    pub due_at: Option<String>,
    pub version: i64,
    pub updated_at: String,
    pub submission_status: Option<String>,
    pub submission_version: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CollectListResponse {
    pub collects: Vec<CollectDto>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateCollectRequest {
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub due_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionDto {
    pub status: String,
    pub version: i64,
    pub payload: serde_json::Value,
    pub submitted_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CollectDetailResponse {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub due_at: Option<String>,
    pub version: i64,
    pub updated_at: String,
    pub submission: Option<SubmissionDto>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveSubmissionRequest {
    pub expected_version: i64,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VersionConflictResponse {
    pub code: String,
    pub message: String,
    pub request_id: String,
    pub current_version: i64,
}
