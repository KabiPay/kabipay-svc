//! Rows in `employment_history` — used by payroll gross (latest `effective_from` wins in v1).

use chrono::{NaiveDate, Utc};
use kabipay_common::{KabiPayError, KabiPayResult};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use uuid::Uuid;

use crate::entities::d0007_employee_core::employment_history;
use crate::services::employee_service;

/// List compensation history for an employee, newest `effective_from` first.
pub async fn list_for_employee(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<employment_history::Model>> {
    let limit = limit.clamp(1, 100);
    employment_history::Entity::find()
        .filter(employment_history::Column::TenantId.eq(tenant_id))
        .filter(employment_history::Column::EmployeeId.eq(employee_id))
        .filter(employment_history::Column::IsDeleted.eq(false))
        .order_by_desc(employment_history::Column::EffectiveFrom)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

/// Set monthly salary for pay run: inserts a row (or updates existing row for the same `effective_from`).
/// Snapshots `department_id` / `designation_id` from the employee at write time.
pub async fn set_monthly_salary(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    actor_user_id: Uuid,
    employee_id: Uuid,
    monthly_salary: Decimal,
    effective_from: NaiveDate,
    change_reason: Option<String>,
) -> KabiPayResult<employment_history::Model> {
    let emp = employee_service::find_by_id(db, tenant_id, employee_id)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "employee",
            id: employee_id.to_string(),
        })?;

    let existing_same_date = employment_history::Entity::find()
        .filter(employment_history::Column::TenantId.eq(tenant_id))
        .filter(employment_history::Column::EmployeeId.eq(employee_id))
        .filter(employment_history::Column::EffectiveFrom.eq(effective_from))
        .filter(employment_history::Column::IsDeleted.eq(false))
        .one(db)
        .await
        .map_err(KabiPayError::from)?;

    let now = Utc::now();

    if let Some(row) = existing_same_date {
        let mut am: employment_history::ActiveModel = row.into();
        am.salary = Set(Some(monthly_salary));
        am.changed_by = Set(Some(actor_user_id));
        am.change_reason = Set(change_reason);
        am.updated_at = Set(now);
        am.department_id = Set(emp.department_id);
        am.designation_id = Set(emp.designation_id);
        am.update(db).await.map_err(KabiPayError::from)
    } else {
        let id = Uuid::new_v4();
        let am = employment_history::ActiveModel {
            id: Set(id),
            tenant_id: Set(tenant_id),
            employee_id: Set(employee_id),
            department_id: Set(emp.department_id),
            designation_id: Set(emp.designation_id),
            cost_center_id: Set(emp.cost_center_id),
            salary: Set(Some(monthly_salary)),
            effective_from: Set(effective_from),
            effective_to: Set(None),
            change_reason: Set(change_reason),
            changed_by: Set(Some(actor_user_id)),
            is_deleted: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
        };
        am.insert(db).await.map_err(KabiPayError::from)?;
        employment_history::Entity::find_by_id(id)
            .one(db)
            .await
            .map_err(KabiPayError::from)?
            .ok_or_else(|| KabiPayError::Internal("inserted employment_history not found".into()))
    }
}
