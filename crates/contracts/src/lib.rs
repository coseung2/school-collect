use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ServiceStatus {
    pub status: &'static str,
    pub service: &'static str,
}
