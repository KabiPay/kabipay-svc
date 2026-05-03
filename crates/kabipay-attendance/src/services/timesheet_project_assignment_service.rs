//! Per-employee whitelist of `TIMESHEET_PROJECT` codes from `master_data`.
//! When an employee has **no** assignment rows, every active catalog project is allowed.

use std::collections::HashSet;

use chrono::Utc;
use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0010_time_shift_roster::timesheet_project_assignment;
use kabipay_db_entities::tenant::d0028_master_data::master_data;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    Set, TransactionTrait,
};
use uuid::Uuid;

use super::hrms_master_service;

pub async fn list_assigned_codes(
    db: &impl ConnectionTrait,
    tenant_id: Uuid,
    employee_id: Uuid,
) -> KabiPayResult<Vec<String>> {
    let rows = timesheet_project_assignment::Entity::find()
        .filter(timesheet_project_assignment::Column::TenantId.eq(tenant_id))
        .filter(timesheet_project_assignment::Column::EmployeeId.eq(employee_id))
        .all(db)
        .await?;
    let mut codes: Vec<String> = rows.into_iter().map(|r| r.project_code).collect();
    codes.sort();
    codes.dedup();
    Ok(codes)
}

/// Replace all assignments for one employee (empty ⇒ remove restrictions).
pub async fn set_assignments_for_employee(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    project_codes: Vec<String>,
    assigned_by_user_id: Option<Uuid>,
) -> KabiPayResult<()> {
    let mut normalized: Vec<String> = project_codes
        .into_iter()
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    normalized.sort();

    let catalog = hrms_master_service::list_projects(db, tenant_id, 500).await?;
    let catalog_set: HashSet<String> = catalog.into_iter().map(|m| m.data_key.to_uppercase()).collect();

    for c in &normalized {
        if !catalog_set.contains(c) {
            return Err(KabiPayError::Validation(format!(
                "unknown or inactive timesheet project code: {c}"
            )));
        }
    }

    let txn = db.begin().await?;

    timesheet_project_assignment::Entity::delete_many()
        .filter(timesheet_project_assignment::Column::TenantId.eq(tenant_id))
        .filter(timesheet_project_assignment::Column::EmployeeId.eq(employee_id))
        .exec(&txn)
        .await?;

    let now = Utc::now();
    for code in normalized {
        let id = Uuid::new_v4();
        let am = timesheet_project_assignment::ActiveModel {
            id: Set(id),
            tenant_id: Set(tenant_id),
            employee_id: Set(employee_id),
            project_code: Set(code),
            assigned_by_user_id: Set(assigned_by_user_id),
            created_at: Set(now),
            updated_at: Set(now),
        };
        am.insert(&txn).await?;
    }

    txn.commit().await?;
    Ok(())
}

pub async fn assert_project_allowed_for_employee(
    db: &impl ConnectionTrait,
    tenant_id: Uuid,
    employee_id: Uuid,
    project_code: Option<&str>,
) -> KabiPayResult<()> {
    let Some(raw) = project_code else {
        return Ok(());
    };
    let pc = raw.trim().to_uppercase();
    if pc.is_empty() {
        return Ok(());
    }

    let catalog = hrms_master_service::list_projects(db, tenant_id, 500).await?;
    let in_catalog = catalog.iter().any(|m| m.data_key.eq_ignore_ascii_case(&pc));
    if !in_catalog {
        return Err(KabiPayError::Validation(format!(
            "unknown timesheet project: {pc}"
        )));
    }

    let assigned = list_assigned_codes(db, tenant_id, employee_id).await?;
    if assigned.is_empty() {
        return Ok(());
    }

    if assigned.iter().any(|c| c.eq_ignore_ascii_case(&pc)) {
        Ok(())
    } else {
        Err(KabiPayError::Validation(format!(
            "project {pc} is not assigned to you — ask your manager or HR to update project assignments"
        )))
    }
}

pub async fn visible_projects_for_employee(
    db: &impl ConnectionTrait,
    tenant_id: Uuid,
    employee_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<master_data::Model>> {
    let catalog = hrms_master_service::list_projects(db, tenant_id, limit).await?;
    let assigned = list_assigned_codes(db, tenant_id, employee_id).await?;
    if assigned.is_empty() {
        return Ok(catalog);
    }
    let set: HashSet<String> = assigned.into_iter().map(|c| c.to_uppercase()).collect();
    Ok(catalog
        .into_iter()
        .filter(|m| set.contains(&m.data_key.to_uppercase()))
        .collect())
}
