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

/// One stored file, as the client sees it. The bytes stay behind the API.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentDto {
    pub id: String,
    pub collect_id: String,
    pub user_id: String,
    pub item_key: String,
    pub file_name: String,
    pub content_type: String,
    pub byte_size: i64,
    pub status: String,
    pub expires_at: String,
    pub stored_at: Option<String>,
    pub created_at: String,
    /// Where the bytes can be read from with the caller's session.
    pub content_url: String,
}

/// Opens one attachment slot. The server decides the object key and the id.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAttachmentRequest {
    pub item_key: String,
    pub file_name: String,
    pub content_type: String,
    pub byte_size: i64,
}

/// How the client should send the bytes for a slot it just opened.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentUploadTarget {
    /// `api` means the client sends the body to `url` with its own session.
    pub kind: String,
    pub url: String,
    pub method: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAttachmentResponse {
    pub attachment: AttachmentDto,
    pub upload: AttachmentUploadTarget,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentListResponse {
    pub attachments: Vec<AttachmentDto>,
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
    /// What the collect asks for. An empty list is allowed while a draft is
    /// still being shaped, but a collect cannot be published without items.
    #[serde(default)]
    pub items: Vec<CollectItemInput>,
    /// Who has to submit. Empty means every member whose role can submit.
    #[serde(default)]
    pub assignee_user_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CollectItemInput {
    /// Stable key used inside a submission payload (`^[a-z][a-z0-9_]*$`).
    pub key: String,
    pub label: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CollectItemDto {
    pub key: String,
    pub label: String,
    pub required: bool,
    pub position: i32,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemberDto {
    pub user_id: String,
    pub display_name: Option<String>,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MemberListResponse {
    pub members: Vec<MemberDto>,
}

/// How many members still owe a submission, and who they are.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CollectStatusRowDto {
    pub user_id: String,
    pub display_name: Option<String>,
    pub role: String,
    pub assignment_status: Option<String>,
    pub submission_status: Option<String>,
    pub submitted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CollectStatusResponse {
    pub assigned: i64,
    pub submitted: i64,
    pub rows: Vec<CollectStatusRowDto>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CollectProgressDto {
    pub assigned: i64,
    pub submitted: i64,
}

/// A collect this caller has to submit, with their own submission state.
#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentDto {
    pub collect_id: String,
    pub title: String,
    pub status: String,
    pub due_at: Option<String>,
    pub assignment_status: String,
    pub submission_status: Option<String>,
    pub submission_version: Option<i64>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssignmentListResponse {
    pub assignments: Vec<AssignmentDto>,
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
    pub items: Vec<CollectItemDto>,
    pub progress: CollectProgressDto,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SaveSubmissionRequest {
    pub expected_version: i64,
    pub payload: serde_json::Value,
}

/// Replaces the item list of a collect. Drafts may be reshaped freely; a
/// published collect may add items and relabel them, but an item that already
/// holds an answer cannot be removed.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCollectItemsRequest {
    /// Collect version the client last observed.
    pub expected_version: i64,
    pub items: Vec<CollectItemInput>,
}

/// Replaces who owes a submission. Removing a target that already saved work is
/// rejected so an answer cannot be orphaned by accident.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCollectAssignmentsRequest {
    pub expected_version: i64,
    #[serde(default)]
    pub assignee_user_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VersionConflictResponse {
    pub code: String,
    pub message: String,
    pub request_id: String,
    pub current_version: i64,
}

/// Invites a teacher to the caller's school. The role is restricted to the
/// non-admin roles so an invitation can never hand out admin rights.
#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateInvitationRequest {
    pub email: String,
    #[serde(default)]
    pub role: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InvitationDto {
    pub id: String,
    pub email: String,
    pub role: String,
    pub status: String,
    pub created_at: String,
    pub expires_at: String,
    pub accepted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InvitationListResponse {
    pub invitations: Vec<InvitationDto>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateInvitationResponse {
    pub invitation: InvitationDto,
    /// The plaintext invite code is returned exactly once, when it is created.
    pub code: String,
}

#[derive(Debug, Clone, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AcceptInvitationRequest {
    pub code: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AcceptInvitationResponse {
    pub membership: MembershipDto,
}
