//! Resolves which tenant users may act on a workflow step (reporting manager vs role).
//!
//! `workflow_step.approver_type`:
//! - `REPORTING_MANAGER` / `MANAGER` / `LINE_MANAGER` — only the subject employee's reporting manager (`employee.user_id`).
//! - `ROLE` — requires `approver_role_id`; user must have that role in `user_role` for the tenant.
//! - `REPORTING_MANAGER_OR_ROLE` — **either** the reporting manager **or** a user assigned `approver_role_id`
//!   (e.g. HR_ADMIN via `user_role`). TEAM-scoped line managers still act only via the reporting-manager branch.

use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::error::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0005_auth_rbac::{permission, role, role_permission, user_role};
use kabipay_db_entities::tenant::d0007_employee_core::employee;
use kabipay_db_entities::tenant::d0025_workflow::workflow_step;
use uuid::Uuid;

fn normalize_approver_type(raw: &Option<String>) -> String {
    match raw {
        None => "REPORTING_MANAGER".to_string(),
        Some(s) => {
            let t = s.trim();
            if t.is_empty() {
                "REPORTING_MANAGER".to_string()
            } else {
                t.to_ascii_uppercase()
            }
        }
    }
}

/// `true` if the user holds `resource`:`action` on any non-deleted tenant role assignment.
pub async fn user_has_permission_via_roles(
    conn: &impl ConnectionTrait,
    tenant_id: Uuid,
    user_id: Uuid,
    resource: &str,
    action: &str,
) -> KabiPayResult<bool> {
    let perm = permission::Entity::find()
        .filter(permission::Column::Resource.eq(resource))
        .filter(permission::Column::Action.eq(action))
        .one(conn)
        .await?;
    let Some(perm) = perm else {
        return Ok(false);
    };

    let assignments = user_role::Entity::find()
        .filter(user_role::Column::UserId.eq(user_id))
        .all(conn)
        .await?;
    if assignments.is_empty() {
        return Ok(false);
    }

    for ur in assignments {
        let Some(r) = role::Entity::find_by_id(ur.role_id)
            .filter(role::Column::TenantId.eq(tenant_id))
            .filter(role::Column::IsDeleted.eq(false))
            .one(conn)
            .await?
        else {
            continue;
        };
        let linked = role_permission::Entity::find()
            .filter(role_permission::Column::RoleId.eq(r.id))
            .filter(role_permission::Column::PermissionId.eq(perm.id))
            .one(conn)
            .await?;
        if linked.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Matched on [`KabiPayError::Forbidden`] for ROLE-only workflow steps when merging timesheet approval.
pub const WORKFLOW_ERR_ROLE_REQUIRED: &str =
    "your account is not assigned the role required for this approval step";

async fn assert_is_reporting_manager_user(
    conn: &impl ConnectionTrait,
    tenant_id: Uuid,
    approver_user_id: Uuid,
    subject_employee_id: Uuid,
) -> KabiPayResult<()> {
    let subj = employee::Entity::find_by_id(subject_employee_id)
        .filter(employee::Column::TenantId.eq(tenant_id))
        .filter(employee::Column::IsDeleted.eq(false))
        .one(conn)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "employee",
            id: subject_employee_id.to_string(),
        })?;
    let Some(mgr_emp_id) = subj.reporting_manager_id else {
        return Err(KabiPayError::Validation(
            "employee has no reporting manager — assign a manager before this approval step can be completed".into(),
        ));
    };
    let mgr = employee::Entity::find_by_id(mgr_emp_id)
        .filter(employee::Column::TenantId.eq(tenant_id))
        .filter(employee::Column::IsDeleted.eq(false))
        .one(conn)
        .await?
        .ok_or_else(|| {
            KabiPayError::Validation("reporting manager employee record not found".into())
        })?;
    match mgr.user_id {
        Some(uid) if uid == approver_user_id => Ok(()),
        Some(_) => Err(KabiPayError::Forbidden(
            "only the employee's reporting manager can approve or reject at this workflow step".into(),
        )),
        None => Err(KabiPayError::Validation(
            "reporting manager has no linked user account — link the manager employee to a login user".into(),
        )),
    }
}

async fn assert_user_has_role(
    conn: &impl ConnectionTrait,
    tenant_id: Uuid,
    approver_user_id: Uuid,
    role_id: Uuid,
) -> KabiPayResult<()> {
    let role_row = role::Entity::find_by_id(role_id)
        .filter(role::Column::TenantId.eq(tenant_id))
        .filter(role::Column::IsDeleted.eq(false))
        .one(conn)
        .await?
        .ok_or_else(|| {
            KabiPayError::Validation("workflow step role not found for this tenant".into())
        })?;

    user_role::Entity::find()
        .filter(user_role::Column::UserId.eq(approver_user_id))
        .filter(user_role::Column::RoleId.eq(role_row.id))
        .one(conn)
        .await?
        .ok_or_else(|| {
            KabiPayError::Forbidden(WORKFLOW_ERR_ROLE_REQUIRED.into())
        })?;
    Ok(())
}

/// Reporting manager **or** a user assigned `fallback_role_id` (e.g. HR_ADMIN).
async fn assert_reporting_manager_or_fallback_role(
    conn: &impl ConnectionTrait,
    tenant_id: Uuid,
    approver_user_id: Uuid,
    subject_employee_id: Uuid,
    fallback_role_id: Uuid,
) -> KabiPayResult<()> {
    if assert_is_reporting_manager_user(
        conn,
        tenant_id,
        approver_user_id,
        subject_employee_id,
    )
    .await
    .is_ok()
    {
        return Ok(());
    }
    if assert_user_has_role(conn, tenant_id, approver_user_id, fallback_role_id)
        .await
        .is_ok()
    {
        return Ok(());
    }
    Err(KabiPayError::Forbidden(
        "only the employee's reporting manager or the workflow fallback role (e.g. HR admin) may approve or reject at this step"
            .into(),
    ))
}

/// Like [`assert_workflow_step_actor`], but for **timesheet week batches** only: if the step is
/// ROLE-only (typical HR second gate) and the approver is not in that role, still allow the
/// subject employee's **reporting manager** (linked `employee.user_id`) to act. This keeps
/// line-manager approval working without a separate HR-only click when the workflow still lists
/// HR as a ROLE step; HR and other assignees still satisfy the primary ROLE check.
pub async fn assert_workflow_step_actor_with_timesheet_reporting_manager_fallback(
    conn: &impl ConnectionTrait,
    tenant_id: Uuid,
    approver_user_id: Uuid,
    subject_employee_id: Uuid,
    step: &workflow_step::Model,
) -> KabiPayResult<()> {
    match assert_workflow_step_actor(
        conn,
        tenant_id,
        approver_user_id,
        subject_employee_id,
        step,
    )
    .await
    {
        Ok(()) => Ok(()),
        Err(KabiPayError::Forbidden(msg)) if msg.contains(WORKFLOW_ERR_ROLE_REQUIRED) => {
            assert_is_reporting_manager_user(
                conn,
                tenant_id,
                approver_user_id,
                subject_employee_id,
            )
            .await
        }
        Err(e) => Err(e),
    }
}

/// Ensures `approver_user_id` may act on `step` for requests from `subject_employee_id`.
pub async fn assert_workflow_step_actor(
    conn: &impl ConnectionTrait,
    tenant_id: Uuid,
    approver_user_id: Uuid,
    subject_employee_id: Uuid,
    step: &workflow_step::Model,
) -> KabiPayResult<()> {
    let kind = normalize_approver_type(&step.approver_type);
    match kind.as_str() {
        "REPORTING_MANAGER" | "MANAGER" | "LINE_MANAGER" => {
            assert_is_reporting_manager_user(
                conn,
                tenant_id,
                approver_user_id,
                subject_employee_id,
            )
            .await
        }
        "ROLE" => {
            let role_id = step.approver_role_id.ok_or_else(|| {
                KabiPayError::Validation(
                    "workflow step uses ROLE approver_type but approver_role_id is missing".into(),
                )
            })?;
            assert_user_has_role(conn, tenant_id, approver_user_id, role_id).await
        }
        "REPORTING_MANAGER_OR_ROLE" | "MANAGER_OR_ROLE" => {
            let role_id = step.approver_role_id.ok_or_else(|| {
                KabiPayError::Validation(
                    "workflow step uses REPORTING_MANAGER_OR_ROLE but approver_role_id is missing (set HR / fallback role)"
                        .into(),
                )
            })?;
            assert_reporting_manager_or_fallback_role(
                conn,
                tenant_id,
                approver_user_id,
                subject_employee_id,
                role_id,
            )
            .await
        }
        other => Err(KabiPayError::Validation(format!(
            "unsupported workflow_step.approver_type: {other}"
        ))),
    }
}

/// Travel requests have no workflow row yet: if the employee has a reporting manager, only that
/// manager may approve/reject; otherwise fall back to `expense`:`approve` (typical HR path).
pub async fn assert_travel_approval_actor(
    conn: &impl ConnectionTrait,
    tenant_id: Uuid,
    approver_user_id: Uuid,
    subject_employee_id: Uuid,
) -> KabiPayResult<()> {
    let subj = employee::Entity::find_by_id(subject_employee_id)
        .filter(employee::Column::TenantId.eq(tenant_id))
        .filter(employee::Column::IsDeleted.eq(false))
        .one(conn)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "employee",
            id: subject_employee_id.to_string(),
        })?;
    if subj.reporting_manager_id.is_some() {
        return assert_is_reporting_manager_user(
            conn,
            tenant_id,
            approver_user_id,
            subject_employee_id,
        )
        .await;
    }
    if user_has_permission_via_roles(conn, tenant_id, approver_user_id, "expense", "approve")
        .await?
    {
        return Ok(());
    }
    Err(KabiPayError::Forbidden(
        "travel approval requires expense approval permission when the employee has no reporting manager".into(),
    ))
}
