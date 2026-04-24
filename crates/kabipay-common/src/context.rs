//! Request contexts injected by auth middleware.
//!
//! Two planes, two contexts. JWTs issued by the two planes MUST NOT be interchangeable
//! (different `iss` claim, different signing secret, validated by different middleware).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

impl ScopeType {
    /// Wider access wins when merging several role rows for the same resource.
    pub fn rank(self) -> u8 {
        match self {
            ScopeType::Self_ => 1,
            ScopeType::Team => 2,
            ScopeType::Department => 3,
            ScopeType::All => 4,
        }
    }

    /// Parse a DB or JWT `scope_type` string (case-insensitive).
    pub fn parse_loose(s: &str) -> Option<Self> {
        match s.trim().to_ascii_uppercase().as_str() {
            "SELF" => Some(ScopeType::Self_),
            "TEAM" => Some(ScopeType::Team),
            "DEPARTMENT" => Some(ScopeType::Department),
            "ALL" => Some(ScopeType::All),
            _ => None,
        }
    }

    pub fn to_wire(self) -> &'static str {
        match self {
            ScopeType::Self_ => "SELF",
            ScopeType::Team => "TEAM",
            ScopeType::Department => "DEPARTMENT",
            ScopeType::All => "ALL",
        }
    }
}

/// `permission` table `resource` values used for `permission_scope` + list filtering.
pub const SCOPE_RES_EMPLOYEE: &str = "employee";
/// Leave requests and balances roll up under the leave module resource.
pub const SCOPE_RES_LEAVE: &str = "leave";
/// Expense claims — list/filter scope (M10); align `permission_scope.resource`.
pub const SCOPE_RES_EXPENSE: &str = "expense";
/// Attendance + timesheet rows — list/filter scope (M10).
pub const SCOPE_RES_ATTENDANCE: &str = "attendance";

/// The caller’s employee row fields needed for `TEAM` / `DEPARTMENT` list filters.
#[derive(Debug, Clone, Copy)]
pub struct ClientViewerEmployee {
    pub employee_id: Uuid,
    pub department_id: Option<Uuid>,
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
        perms
            .iter()
            .any(|p| self.permissions.iter().any(|owned| owned == p))
    }

    /// Returns the effective scope for a resource, defaulting to `Self_` if no scope is defined.
    pub fn scope_for(&self, resource: &str) -> ScopeType {
        self.scopes
            .get(resource)
            .copied()
            .unwrap_or(ScopeType::Self_)
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
    /// Widest `ScopeType` per `permission.resource` (keys: e.g. `employee`, `leave` — wire values
    /// are `SELF` | `TEAM` | `DEPARTMENT` | `ALL`). Omitted in legacy tokens; treated as SELF.
    #[serde(default)]
    pub resource_scopes: HashMap<String, String>,
}

pub const OPERATOR_JWT_ISSUER: &str = "kabipay-ops";
pub const CLIENT_JWT_ISSUER: &str = "kabipay-client";

/// JWT `permissions` claim uses `resource:action` to match `permission` rows.
pub const PERM_EMPLOYEE_WRITE: &str = "employee:write";
/// Broader org directory edits (e.g. bulk / sensitive fields) — same gate as write for now.
pub const PERM_EMPLOYEE_MANAGE: &str = "employee:manage";
/// Approve or reject other users' leave requests.
pub const PERM_LEAVE_APPROVE: &str = "leave:approve";
/// Approve or reject expense claims submitted by others.
pub const PERM_EXPENSE_APPROVE: &str = "expense:approve";
/// Approve or reject **tax proof** lines (submitted actuals vs declared deductions).
pub const PERM_TAX_PROOF_APPROVE: &str = "tax:approve";
/// Export India payroll statutory artefacts (e.g. monthly TDS summary CSV) for the tenant.
pub const PERM_PAYROLL_STATUTORY_EXPORT: &str = "payroll:statutory_export";
/// Configure live punch enforcement (geofence / IP allowlist) for the tenant.
pub const PERM_ATTENDANCE_PUNCH_POLICY: &str = "attendance:punch_policy";

/// HTTP-derived metadata attached to each GraphQL request by [`crate::subgraph::tenant_graphql_post`].
/// Values come from gateway headers, not from GraphQL variables (so they are suitable for policy).
#[derive(Clone, Debug, Default)]
pub struct ClientRequestHints {
    /// First hop from `X-Forwarded-For`, else `X-Real-IP`, when present.
    pub client_ip: Option<String>,
}

impl ClientClaims {
    /// True if the token includes one of the permission strings (exact match on wire).
    pub fn has_any_permission(&self, perms: &[&str]) -> bool {
        perms
            .iter()
            .any(|p| self.permissions.iter().any(|owned| owned == p))
    }

    /// Create/update other users' **employee** rows (not self-service profile edits).
    pub fn can_manage_employee_directory(&self) -> bool {
        if self.has_any_permission(&[PERM_EMPLOYEE_WRITE, PERM_EMPLOYEE_MANAGE]) {
            return true;
        }
        self.roles.iter().any(|r| {
            let u = r.to_ascii_uppercase();
            u == "HR_ADMIN" || u == "TENANT_ADMIN" || u == "ORG_ADMIN"
        })
    }

    /// Approve or reject **leave** requests (not the employee's own self-service only path).
    pub fn can_approve_leave(&self) -> bool {
        if self.has_any_permission(&[PERM_LEAVE_APPROVE]) {
            return true;
        }
        self.roles.iter().any(|r| {
            let u = r.to_ascii_uppercase();
            u == "HR_ADMIN" || u == "TENANT_ADMIN" || u == "ORG_ADMIN"
        })
    }

    /// Approve or reject **expense** claims (approver/manager path).
    pub fn can_approve_expense(&self) -> bool {
        if self.has_any_permission(&[PERM_EXPENSE_APPROVE]) {
            return true;
        }
        self.roles.iter().any(|r| {
            let u = r.to_ascii_uppercase();
            u == "HR_ADMIN" || u == "TENANT_ADMIN" || u == "ORG_ADMIN"
        })
    }

    /// Approve or reject **tax deduction proof** lines (documented actuals).
    pub fn can_approve_tax_proof(&self) -> bool {
        if self.has_any_permission(&[PERM_TAX_PROOF_APPROVE]) {
            return true;
        }
        self.roles.iter().any(|r| {
            let u = r.to_ascii_uppercase();
            u == "HR_ADMIN" || u == "TENANT_ADMIN" || u == "ORG_ADMIN"
        })
    }

    /// Download tenant-wide **statutory payroll** reports (TDS summary CSV, etc.).
    pub fn can_export_payroll_statutory(&self) -> bool {
        if self.has_any_permission(&[PERM_PAYROLL_STATUTORY_EXPORT]) {
            return true;
        }
        self.roles.iter().any(|r| {
            let u = r.to_ascii_uppercase();
            u == "HR_ADMIN" || u == "TENANT_ADMIN" || u == "ORG_ADMIN"
        })
    }

    /// Configure **live punch** policy (geofence + IP allowlist) for the tenant.
    pub fn can_configure_attendance_punch_policy(&self) -> bool {
        if self.has_any_permission(&[PERM_ATTENDANCE_PUNCH_POLICY]) {
            return true;
        }
        self.roles.iter().any(|r| {
            let u = r.to_ascii_uppercase();
            u == "HR_ADMIN" || u == "TENANT_ADMIN" || u == "ORG_ADMIN"
        })
    }

    /// Effective data scope for list/detail filters (`permission_scope` merged at login). Defaults
    /// to `Self_` when unset (legacy tokens and least-privilege default).
    pub fn data_scope(&self, resource: &str) -> ScopeType {
        self.resource_scopes
            .get(resource)
            .and_then(|s| ScopeType::parse_loose(s))
            .unwrap_or(ScopeType::Self_)
    }
}
