//! Newtype wrappers for canonical domain identifiers.
//!
//! Per the cross-service identifier contract (Gap A), these IDs are the ONLY
//! employee/tenant/user/department references services should store. Never duplicate
//! core fields (name, email, department name) — resolve them via GraphQL federation.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! newtype_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(pub Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn inner(&self) -> Uuid {
                self.0
            }
        }

        impl From<Uuid> for $name {
            fn from(u: Uuid) -> Self {
                Self(u)
            }
        }

        impl From<$name> for Uuid {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

newtype_id!(
    TenantId,
    "Canonical tenant identifier. Same value across all services."
);
newtype_id!(UserId, "Canonical user identifier (client plane).");
newtype_id!(
    EmployeeId,
    "Canonical employee identifier. THE cross-service contract."
);
newtype_id!(DepartmentId, "Canonical department identifier.");
newtype_id!(
    OperatorUserId,
    "Operator-plane user identifier (isolated from UserId)."
);
