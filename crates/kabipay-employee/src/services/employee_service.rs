//! Employee queries and write operations on a tenant-scoped connection.
//!
//! Every query applies both the `tenant_id` filter (Gap A — defence in depth even with
//! schema isolation) and the `is_deleted = false` filter (Gap B — soft-delete policy).

use chrono::{NaiveDate, Utc};
use kabipay_common::client_data_scope::employee_model_in_scope;
use kabipay_common::context::ClientViewerEmployee;
use kabipay_common::context::ScopeType;
use kabipay_common::{KabiPayError, KabiPayResult};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use std::collections::HashMap;
use uuid::Uuid;

use crate::entities::d0007_employee_core::employee;

/// `new_manager` must exist, differ from `subject_employee_id`, and must not create a reporting loop.
pub async fn assert_valid_reporting_manager(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    subject_employee_id: Uuid,
    new_manager_id: Uuid,
) -> KabiPayResult<()> {
    if subject_employee_id == new_manager_id {
        return Err(KabiPayError::Validation(
            "an employee cannot report to themselves".into(),
        ));
    }
    find_by_id(db, tenant_id, new_manager_id)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "employee",
            id: new_manager_id.to_string(),
        })?;

    let mut current = new_manager_id;
    for _ in 0..64 {
        let row = find_by_id(db, tenant_id, current)
            .await?
            .ok_or_else(|| KabiPayError::Internal("reporting chain broke".into()))?;
        let Some(mid) = row.reporting_manager_id else {
            break;
        };
        if mid == subject_employee_id {
            return Err(KabiPayError::Validation(
                "that reporting manager would create a loop in the org chart".into(),
            ));
        }
        current = mid;
    }
    Ok(())
}

/// Look up one non-deleted employee inside a tenant schema.
///
/// Returns `Ok(None)` when the employee is not found (or is soft-deleted / belongs
/// to another tenant) so the resolver can render a nullable `Employee`.
pub async fn find_by_id(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
) -> KabiPayResult<Option<employee::Model>> {
    employee::Entity::find_by_id(employee_id)
        .filter(employee::Column::TenantId.eq(tenant_id))
        .filter(employee::Column::IsDeleted.eq(false))
        .one(db)
        .await
        .map_err(KabiPayError::from)
}

/// Full display names for referenced employees (e.g. reporting manager labels).
pub async fn map_full_names(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    ids: &[Uuid],
) -> KabiPayResult<HashMap<Uuid, String>> {
    if ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = employee::Entity::find()
        .filter(employee::Column::TenantId.eq(tenant_id))
        .filter(employee::Column::IsDeleted.eq(false))
        .filter(employee::Column::Id.is_in(ids.to_vec()))
        .all(db)
        .await
        .map_err(KabiPayError::from)?;
    Ok(rows
        .into_iter()
        .map(|m| {
            let full_name = format!("{} {}", m.first_name.trim(), m.last_name.trim())
                .trim()
                .to_string();
            (m.id, full_name)
        })
        .collect())
}

/// Whether a fetched employee row is visible under `scope` (used for `employee(id:)` / IDOR checks).
pub fn is_employee_in_scope(
    scope: ScopeType,
    viewer: Option<ClientViewerEmployee>,
    target: &employee::Model,
) -> bool {
    employee_model_in_scope(scope, viewer, target)
}

/// List the first `limit` non-deleted employees, filtered by the caller’s data scope
/// (`ALL` = entire tenant, otherwise `scope` + `viewer`).
///
/// `limit` is clamped to the range `1..=100` so a caller cannot force a full-table scan.
/// When the scope is not `All` and `viewer` is missing (no linked employee), returns an empty list.
pub async fn list(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
    scope: ScopeType,
    viewer: Option<ClientViewerEmployee>,
) -> KabiPayResult<Vec<employee::Model>> {
    let limit = limit.clamp(1, 100);
    let mut q = employee::Entity::find()
        .filter(employee::Column::TenantId.eq(tenant_id))
        .filter(employee::Column::IsDeleted.eq(false));

    match scope {
        ScopeType::All => {}
        ScopeType::Self_ => {
            let Some(v) = viewer else {
                return Ok(vec![]);
            };
            q = q.filter(employee::Column::Id.eq(v.employee_id));
        }
        ScopeType::Team => {
            let Some(v) = viewer else {
                return Ok(vec![]);
            };
            q = q.filter(
                Condition::any()
                    .add(employee::Column::Id.eq(v.employee_id))
                    .add(employee::Column::ReportingManagerId.eq(v.employee_id)),
            );
        }
        ScopeType::Department => {
            let Some(v) = viewer else {
                return Ok(vec![]);
            };
            q = if let Some(d) = v.department_id {
                q.filter(
                    Condition::any()
                        .add(employee::Column::Id.eq(v.employee_id))
                        .add(employee::Column::DepartmentId.eq(Some(d))),
                )
            } else {
                q.filter(employee::Column::Id.eq(v.employee_id))
            };
        }
    }

    q.limit(limit).all(db).await.map_err(KabiPayError::from)
}

/// Employees visible under the same data scope as [`list`], with a higher row cap for org-chart views.
/// `limit` is clamped to `1..=500`.
pub async fn list_for_org_chart(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
    scope: ScopeType,
    viewer: Option<ClientViewerEmployee>,
) -> KabiPayResult<Vec<employee::Model>> {
    let limit = limit.clamp(1, 500);
    let mut q = employee::Entity::find()
        .filter(employee::Column::TenantId.eq(tenant_id))
        .filter(employee::Column::IsDeleted.eq(false));

    match scope {
        ScopeType::All => {}
        ScopeType::Self_ => {
            let Some(v) = viewer else {
                return Ok(vec![]);
            };
            q = q.filter(employee::Column::Id.eq(v.employee_id));
        }
        ScopeType::Team => {
            let Some(v) = viewer else {
                return Ok(vec![]);
            };
            q = q.filter(
                Condition::any()
                    .add(employee::Column::Id.eq(v.employee_id))
                    .add(employee::Column::ReportingManagerId.eq(v.employee_id)),
            );
        }
        ScopeType::Department => {
            let Some(v) = viewer else {
                return Ok(vec![]);
            };
            q = if let Some(d) = v.department_id {
                q.filter(
                    Condition::any()
                        .add(employee::Column::Id.eq(v.employee_id))
                        .add(employee::Column::DepartmentId.eq(Some(d))),
                )
            } else {
                q.filter(employee::Column::Id.eq(v.employee_id))
            };
        }
    }

    q.order_by_asc(employee::Column::EmployeeCode)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

/// Payload for a new `employee` row (no GraphQL types here).
pub struct NewEmployee {
    pub employee_code: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_joining: NaiveDate,
    pub department_id: Option<Uuid>,
    pub designation_id: Option<Uuid>,
    pub reporting_manager_id: Option<Uuid>,
    pub employment_type: Option<String>,
    pub status: String,
    pub user_id: Option<Uuid>,
}

pub async fn create(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    data: NewEmployee,
) -> KabiPayResult<employee::Model> {
    if employee::Entity::find()
        .filter(employee::Column::TenantId.eq(tenant_id))
        .filter(employee::Column::EmployeeCode.eq(&data.employee_code))
        .one(db)
        .await?
        .is_some()
    {
        return Err(KabiPayError::Conflict(
            "employee code is already in use in this tenant".into(),
        ));
    }

    let id = Uuid::new_v4();
    if let Some(mgr) = data.reporting_manager_id {
        assert_valid_reporting_manager(db, tenant_id, id, mgr).await?;
    }
    let now = Utc::now();
    let am = employee::ActiveModel {
        id: Set(id),
        tenant_id: Set(tenant_id),
        user_id: Set(data.user_id),
        department_id: Set(data.department_id),
        designation_id: Set(data.designation_id),
        cost_center_id: Set(None),
        location_id: Set(None),
        reporting_manager_id: Set(data.reporting_manager_id),
        employee_code: Set(data.employee_code),
        first_name: Set(data.first_name),
        last_name: Set(data.last_name),
        date_of_birth: Set(None),
        gender: Set(None),
        blood_group: Set(None),
        nationality: Set(None),
        employment_type: Set(data.employment_type),
        status: Set(data.status),
        date_of_joining: Set(data.date_of_joining),
        probation_end_date: Set(None),
        notice_period_days: Set(None),
        emergency_contact_name: Set(None),
        emergency_contact_phone: Set(None),
        emergency_contact_relation: Set(None),
        uan_number: Set(None),
        esic_number: Set(None),
        is_deleted: Set(false),
        deleted_at: Set(None),
        deleted_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    am.insert(db).await?;
    employee::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("inserted employee not found".into()))
}

/// Partial update: each `Some` field replaces the column; `None` = leave unchanged.
pub struct EmployeePatch {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub department_id: Option<Uuid>,
    pub designation_id: Option<Uuid>,
    /// `None` = do not change; `Some(None)` = clear; `Some(Some(u))` = set (validated).
    pub reporting_manager_id: Option<Option<Uuid>>,
    pub employment_type: Option<String>,
    pub status: Option<String>,
    pub user_id: Option<Uuid>,
}

pub async fn update(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    patch: EmployeePatch,
) -> KabiPayResult<employee::Model> {
    let existing = find_by_id(db, tenant_id, employee_id)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "employee",
            id: employee_id.to_string(),
        })?;
    let mut am: employee::ActiveModel = existing.into();
    if let Some(v) = patch.first_name {
        am.first_name = Set(v);
    }
    if let Some(v) = patch.last_name {
        am.last_name = Set(v);
    }
    if let Some(v) = patch.department_id {
        am.department_id = Set(Some(v));
    }
    if let Some(v) = patch.designation_id {
        am.designation_id = Set(Some(v));
    }
    if let Some(inner) = patch.reporting_manager_id {
        match inner {
            None => {
                am.reporting_manager_id = Set(None);
            }
            Some(mgr) => {
                assert_valid_reporting_manager(db, tenant_id, employee_id, mgr).await?;
                am.reporting_manager_id = Set(Some(mgr));
            }
        }
    }
    if let Some(v) = patch.employment_type {
        am.employment_type = Set(Some(v));
    }
    if let Some(v) = patch.status {
        am.status = Set(v);
    }
    if let Some(v) = patch.user_id {
        am.user_id = Set(Some(v));
    }
    am.updated_at = Set(Utc::now());
    am.update(db).await?;
    find_by_id(db, tenant_id, employee_id)
        .await?
        .ok_or_else(|| KabiPayError::Internal("updated employee not found".into()))
}
