use school_collect_domain::{TenantId, UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Actor {
    pub user_id: UserId,
    pub tenant_id: TenantId,
}

// OIDC verification and dynamic authorization are implemented in Stage 6.
// This crate is the server-side trust-boundary seam; clients never authorize
// themselves by supplying a role.
