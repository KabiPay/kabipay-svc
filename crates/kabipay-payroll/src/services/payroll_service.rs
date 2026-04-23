//! Tenant-scoped SeaORM queries for payroll catalog and cycles.

use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0012_payroll::{payroll_cycle, payslip, payslip_component, salary_component};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use std::collections::HashMap;
use uuid::Uuid;

pub async fn list_components(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    active_only: bool,
    limit: u64,
) -> KabiPayResult<Vec<salary_component::Model>> {
    let limit = limit.clamp(1, 200);
    let mut q = salary_component::Entity::find()
        .filter(salary_component::Column::TenantId.eq(tenant_id));
    if active_only {
        q = q.filter(salary_component::Column::IsActive.eq(true));
    }
    q.order_by_asc(salary_component::Column::Code)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_cycles(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<payroll_cycle::Model>> {
    let limit = limit.clamp(1, 60);
    payroll_cycle::Entity::find()
        .filter(payroll_cycle::Column::TenantId.eq(tenant_id))
        .order_by_desc(payroll_cycle::Column::Year)
        .order_by_desc(payroll_cycle::Column::Month)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

/// Payslips for a tenant, optionally restricted to one employee, newest first.
pub async fn list_payslips(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Option<Uuid>,
    limit: u64,
) -> KabiPayResult<Vec<payslip::Model>> {
    let limit = limit.clamp(1, 60);
    let mut q = payslip::Entity::find().filter(payslip::Column::TenantId.eq(tenant_id));
    if let Some(e) = employee_id {
        q = q.filter(payslip::Column::EmployeeId.eq(e));
    }
    q.order_by_desc(payslip::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

/// One payslip with its `payslip_component` rows.
pub async fn find_payslip_detail(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    id: Uuid,
) -> KabiPayResult<Option<(payslip::Model, Vec<payslip_component::Model>)>> {
    let Some(m) = payslip::Entity::find()
        .filter(payslip::Column::Id.eq(id))
        .filter(payslip::Column::TenantId.eq(tenant_id))
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    let comps = payslip_component::Entity::find()
        .filter(payslip_component::Column::TenantId.eq(tenant_id))
        .filter(payslip_component::Column::PayslipId.eq(id))
        .order_by_asc(payslip_component::Column::CreatedAt)
        .all(db)
        .await?;
    Ok(Some((m, comps)))
}

/// Batch lines for all payslips returned by [`list_payslips`].
pub async fn payslip_lines_by_payslip_ids(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    payslip_ids: &[Uuid],
) -> KabiPayResult<HashMap<Uuid, Vec<payslip_component::Model>>> {
    if payslip_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let comps = payslip_component::Entity::find()
        .filter(payslip_component::Column::TenantId.eq(tenant_id))
        .filter(payslip_component::Column::PayslipId.is_in(payslip_ids.to_vec()))
        .all(db)
        .await?;
    let mut map: HashMap<Uuid, Vec<payslip_component::Model>> = HashMap::new();
    for c in comps {
        map.entry(c.payslip_id).or_default().push(c);
    }
    Ok(map)
}
