//! Tenant-scoped SeaORM queries for payroll catalog and cycles.

use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0007_employee_core::{employee, employee_pan};
use kabipay_db_entities::tenant::d0012_payroll::{
    payroll_cycle, payslip, payslip_component, salary_component,
};
use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub async fn list_components(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    active_only: bool,
    limit: u64,
) -> KabiPayResult<Vec<salary_component::Model>> {
    let limit = limit.clamp(1, 200);
    let mut q =
        salary_component::Entity::find().filter(salary_component::Column::TenantId.eq(tenant_id));
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

fn csv_cell(raw: &str) -> String {
    if raw.contains(',') || raw.contains('"') || raw.contains('\n') || raw.contains('\r') {
        format!("\"{}\"", raw.replace('"', "\"\""))
    } else {
        raw.to_string()
    }
}

fn dec_cell(d: Decimal) -> String {
    csv_cell(&d.normalize().to_string())
}

/// India payroll stub: one CSV listing all payslips in a payroll cycle (month + year) with TDS and PAN.
/// Header is always present; body is empty when no matching cycle or no payslips.
pub async fn india_tds_monthly_summary_csv(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    month: i32,
    year: i32,
) -> KabiPayResult<String> {
    if !(1..=12).contains(&month) || !(1900..=2200).contains(&year) {
        return Err(KabiPayError::Validation(
            "month must be 1–12 and year a plausible calendar year".into(),
        ));
    }

    let cycle = payroll_cycle::Entity::find()
        .filter(payroll_cycle::Column::TenantId.eq(tenant_id))
        .filter(payroll_cycle::Column::Month.eq(month))
        .filter(payroll_cycle::Column::Year.eq(year))
        .one(db)
        .await
        .map_err(KabiPayError::from)?;

    let mut out = String::from(
        "employee_code,employee_name,pan,period_month,period_year,payroll_cycle_name,gross_salary,total_deductions,tds_amount,net_salary,payslip_status,payslip_id\n",
    );

    let Some(cycle_row) = cycle else {
        return Ok(out);
    };

    let slips = payslip::Entity::find()
        .filter(payslip::Column::TenantId.eq(tenant_id))
        .filter(payslip::Column::PayrollCycleId.eq(cycle_row.id))
        .order_by_asc(payslip::Column::EmployeeId)
        .all(db)
        .await
        .map_err(KabiPayError::from)?;

    if slips.is_empty() {
        return Ok(out);
    }

    let emp_ids: Vec<Uuid> = slips.iter().map(|p| p.employee_id).collect();
    let employees = employee::Entity::find()
        .filter(employee::Column::TenantId.eq(tenant_id))
        .filter(employee::Column::IsDeleted.eq(false))
        .filter(employee::Column::Id.is_in(emp_ids.clone()))
        .all(db)
        .await
        .map_err(KabiPayError::from)?;
    let emp_map: HashMap<Uuid, employee::Model> =
        employees.into_iter().map(|e| (e.id, e)).collect();

    let pans = employee_pan::Entity::find()
        .filter(employee_pan::Column::TenantId.eq(tenant_id))
        .filter(employee_pan::Column::EmployeeId.is_in(emp_ids))
        .filter(employee_pan::Column::IsPrimary.eq(true))
        .all(db)
        .await
        .map_err(KabiPayError::from)?;
    let mut pan_by_emp: HashMap<Uuid, String> = HashMap::new();
    let mut seen: HashSet<Uuid> = HashSet::new();
    for p in pans {
        if seen.insert(p.employee_id) {
            pan_by_emp.insert(p.employee_id, p.pan_number);
        }
    }

    let cycle_name = &cycle_row.name;
    for p in slips {
        let (code, name) = match emp_map.get(&p.employee_id) {
            Some(e) => (
                e.employee_code.as_str(),
                format!("{} {}", e.first_name, e.last_name),
            ),
            None => ("", String::new()),
        };
        let pan = pan_by_emp
            .get(&p.employee_id)
            .map(String::as_str)
            .unwrap_or("");
        let tds = p.tds_amount.unwrap_or(Decimal::ZERO);
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{}\n",
            csv_cell(code),
            csv_cell(&name),
            csv_cell(pan),
            month,
            year,
            csv_cell(cycle_name),
            dec_cell(p.gross_salary),
            dec_cell(p.total_deductions),
            dec_cell(tds),
            dec_cell(p.net_salary),
            csv_cell(&p.status),
            csv_cell(&p.id.to_string()),
        ));
    }

    Ok(out)
}

/// India payroll stub: PF + ESI columns from `payslip` for all rows in the payroll cycle (`month` + `year`).
/// Same RBAC as TDS export; not an ECR / challan file — statutory prep only.
pub async fn india_pf_esi_monthly_summary_csv(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    month: i32,
    year: i32,
) -> KabiPayResult<String> {
    if !(1..=12).contains(&month) || !(1900..=2200).contains(&year) {
        return Err(KabiPayError::Validation(
            "month must be 1–12 and year a plausible calendar year".into(),
        ));
    }

    let cycle = payroll_cycle::Entity::find()
        .filter(payroll_cycle::Column::TenantId.eq(tenant_id))
        .filter(payroll_cycle::Column::Month.eq(month))
        .filter(payroll_cycle::Column::Year.eq(year))
        .one(db)
        .await
        .map_err(KabiPayError::from)?;

    let mut out = String::from(
        "employee_code,employee_name,pan,uan_number,esic_number,period_month,period_year,payroll_cycle_name,pf_employee,pf_employer,esi_employee,esi_employer,gross_salary,payslip_status,payslip_id\n",
    );

    let Some(cycle_row) = cycle else {
        return Ok(out);
    };

    let slips = payslip::Entity::find()
        .filter(payslip::Column::TenantId.eq(tenant_id))
        .filter(payslip::Column::PayrollCycleId.eq(cycle_row.id))
        .order_by_asc(payslip::Column::EmployeeId)
        .all(db)
        .await
        .map_err(KabiPayError::from)?;

    if slips.is_empty() {
        return Ok(out);
    }

    let emp_ids: Vec<Uuid> = slips.iter().map(|p| p.employee_id).collect();
    let employees = employee::Entity::find()
        .filter(employee::Column::TenantId.eq(tenant_id))
        .filter(employee::Column::IsDeleted.eq(false))
        .filter(employee::Column::Id.is_in(emp_ids.clone()))
        .all(db)
        .await
        .map_err(KabiPayError::from)?;
    let emp_map: HashMap<Uuid, employee::Model> =
        employees.into_iter().map(|e| (e.id, e)).collect();

    let pans = employee_pan::Entity::find()
        .filter(employee_pan::Column::TenantId.eq(tenant_id))
        .filter(employee_pan::Column::EmployeeId.is_in(emp_ids))
        .filter(employee_pan::Column::IsPrimary.eq(true))
        .all(db)
        .await
        .map_err(KabiPayError::from)?;
    let mut pan_by_emp: HashMap<Uuid, String> = HashMap::new();
    let mut seen: HashSet<Uuid> = HashSet::new();
    for p in pans {
        if seen.insert(p.employee_id) {
            pan_by_emp.insert(p.employee_id, p.pan_number);
        }
    }

    let cycle_name = &cycle_row.name;
    let z = Decimal::ZERO;
    for p in slips {
        let (code, name) = match emp_map.get(&p.employee_id) {
            Some(e) => (
                e.employee_code.as_str(),
                format!("{} {}", e.first_name, e.last_name),
            ),
            None => ("", String::new()),
        };
        let pan = pan_by_emp
            .get(&p.employee_id)
            .map(String::as_str)
            .unwrap_or("");
        let uan = p.uan_number.as_deref().unwrap_or("");
        let esic = p.esic_number.as_deref().unwrap_or("");
        let pf_e = p.pf_employee.unwrap_or(z);
        let pf_r = p.pf_employer.unwrap_or(z);
        let esi_e = p.esi_employee.unwrap_or(z);
        let esi_r = p.esi_employer.unwrap_or(z);
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            csv_cell(code),
            csv_cell(&name),
            csv_cell(pan),
            csv_cell(uan),
            csv_cell(esic),
            month,
            year,
            csv_cell(cycle_name),
            dec_cell(pf_e),
            dec_cell(pf_r),
            dec_cell(esi_e),
            dec_cell(esi_r),
            dec_cell(p.gross_salary),
            csv_cell(&p.status),
            csv_cell(&p.id.to_string()),
        ));
    }

    Ok(out)
}
