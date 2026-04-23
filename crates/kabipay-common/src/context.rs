//! Request contexts injected by auth middleware.
//!
//! Two planes, two contexts. JWTs issued by the two planes MUST NOT be interchangeable
//! (different `iss` claim, different signing secret, validated by different middleware).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Data-level access control scope. Applied per resource per role via `PERMISSION_SCOPE`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ScopeType {
    /// User can only see/edit their own records.
    Self_,
    /// Manager can see their direct reports (resolved via EMPLOYEE_HIERARCHY).
    Team,
    /// HR user can see everyone in their department.
    Department,
    /// Unrestricted within tenant (HR admin, payroll admin).
    All,
}

/// Context attached to every operator-plane request after `operator_auth` middleware runs.
/// Isolated from `ClientContext` — the two must never be interchangeable.
#[derive(Debug, Clone)]
pub struct OperatorContext {
    pub operator_user_id: Uuid,
    pub roles: Vec<String>,
    /// Tenants this operator has scoped access to. Empty vector = super admin (all tenants).
    pub tenant_access: Vec<Uuid>,
}

impl OperatorContext {
    pub fn is_super_admin(&self) -> bool {
        self.tenant_access.is_empty()
    }

    pub fn can_access_tenant(&self, tenant_id: Uuid) -> bool {
        self.is_super_admin() || self.tenant_access.contains(&tenant_id)
    }
}

/// Context attached to every client-plane request after `client_auth` middleware runs.
///
/// ALWAYS contains `tenant_id`. Every SeaORM query in a client service MUST filter by
/// this tenant_id — even though schema isolation already protects, it's defense in depth.
#[derive(Debug, Clone)]
pub struct ClientContext {
    pub user_id: Uuid,
    pub tenant_id: Uuid,
    /// Resolved EMPLOYEE.id if the user is linked to an employee record.
    pub employee_id: Option<Uuid>,
    pub roles: Vec<String>,
    pub permissions: Vec<String>,
    /// Per-resource scope map: resource => ScopeType.
    /// Resolvers apply this to filter queries before returning data.
    pub scopes: std::collections::HashMap<String, ScopeType>,
}

impl ClientContext {
    /// Returns `true` if the user has any of the provided permissions (OR semantics).
    pub fn has_any_permission(&self, perms: &[&str]) -> bool {
        perms.iter().any(|p| self.permissions.iter().any(|owned| owned == p))
    }

    /// Returns the effective scope for a resource, defaulting to `Self_` if no scope is defined.
    pub fn scope_for(&self, resource: &str) -> ScopeType {
        self.scopes.get(resource).copied().unwrap_or(ScopeType::Self_)
    }
}

/// JWT claims for an operator token.
///
/// `roles` / `tenant_access` default to empty so tokens issued by an early
/// version of `kabipay-auth` (before RBAC is fully wired) still round-trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperatorClaims {
    pub sub: Uuid,
    pub iss: String,
    pub exp: i64,
    pub iat: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub email: String,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub tenant_access: Vec<Uuid>,
}

/// JWT claims for a client token.
///
/// `employee_id` / `roles` / `permissions` default to empty / None so
/// tokens issued by an early version of `kabipay-auth` (before RBAC is
/// fully wired) still round-trip.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientClaims {
    pub sub: Uuid,
    pub iss: String,
    pub exp: i64,
    pub iat: i64,
    pub tenant_id: Uuid,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub email: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub employee_id: Option<Uuid>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub permissions: Vec<String>,
}

pub const OPERATOR_JWT_ISSUER: &str = "kabipay-ops";
pub const CLIENT_JWT_ISSUER: &str = "kabipay-client";
