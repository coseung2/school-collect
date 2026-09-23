use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::now_v7())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

id_type!(TenantId);
id_type!(UserId);
id_type!(CollectId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipRole {
    Admin,
    Coordinator,
    Contributor,
    Viewer,
}

impl MembershipRole {
    pub const fn can_manage_collects(self) -> bool {
        matches!(self, Self::Admin | Self::Coordinator)
    }

    pub const fn can_submit(self) -> bool {
        matches!(self, Self::Admin | Self::Coordinator | Self::Contributor)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectStatus {
    Draft,
    Published,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectTransitionError {
    InvalidTransition,
}

impl CollectStatus {
    pub const fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::Published) | (Self::Published, Self::Closed)
        )
    }

    pub const fn transition_to(self, next: Self) -> Result<Self, CollectTransitionError> {
        if self.can_transition_to(next) {
            Ok(next)
        } else {
            Err(CollectTransitionError::InvalidTransition)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CollectStatus, MembershipRole};

    #[test]
    fn collect_status_only_moves_forward() {
        assert_eq!(
            CollectStatus::Draft.transition_to(CollectStatus::Published),
            Ok(CollectStatus::Published)
        );
        assert_eq!(
            CollectStatus::Published.transition_to(CollectStatus::Closed),
            Ok(CollectStatus::Closed)
        );
        assert!(
            CollectStatus::Closed
                .transition_to(CollectStatus::Published)
                .is_err()
        );
    }

    #[test]
    fn roles_encode_server_side_capabilities() {
        assert!(MembershipRole::Admin.can_manage_collects());
        assert!(MembershipRole::Coordinator.can_manage_collects());
        assert!(MembershipRole::Contributor.can_submit());
        assert!(!MembershipRole::Viewer.can_submit());
    }
}
