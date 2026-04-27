//! Tenant-scoped SeaORM queries for payroll catalog and cycles.

use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0007_employee_core::{
    employee, employee_bank, employee_pan, employment_history,
};
use kabipay_db_entities::tenant::d0012_payroll::{
    payroll_cycle, payslip, payslip_component, salary_component,
};
use kabipay_db_entities::tenant::d0013_tax_statutory::tax_computation;
use chrono::{NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::services::arrear_service;
use crate::services::statutory_india;

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

/// Insert a **DRAFT** `payroll_cycle` for a calendar month. Rejects if a cycle for the same
/// tenant + month + year already exists.
pub async fn create_payroll_cycle(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    name: String,
    month: i32,
    year: i32,
    payment_date: Option<NaiveDate>,
) -> KabiPayResult<payroll_cycle::Model> {
    let name = name.trim();
    if name.is_empty() {
        return Err(KabiPayError::Validation("name must not be empty".into()));
    }
    if !(1..=12).contains(&month) {
        return Err(KabiPayError::Validation("month must be 1–12".into()));
    }
    if !(2000..=2200).contains(&year) {
        return Err(KabiPayError::Validation("year must be between 2000 and 2200".into()));
    }

    let existing = payroll_cycle::Entity::find()
        .filter(payroll_cycle::Column::TenantId.eq(tenant_id))
        .filter(payroll_cycle::Column::Month.eq(month))
        .filter(payroll_cycle::Column::Year.eq(year))
        .one(db)
        .await
        .map_err(KabiPayError::from)?;
    if existing.is_some() {
        return Err(KabiPayError::Validation(format!(
            "a payroll cycle already exists for {month:02}/{year}"
        )));
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    let inserted = payroll_cycle::ActiveModel {
        id: Set(id),
        tenant_id: Set(tenant_id),
        name: Set(name.to_string()),
        month: Set(month),
        year: Set(year),
        status: Set("DRAFT".to_string()),
        payment_date: Set(payment_date),
        processed_by: Set(None),
        processed_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(KabiPayError::from)?;
    Ok(inserted)
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

/// **Bank disbursement (CSV).** One row per payslip in the payroll cycle for `month` + `year`, with
/// the employee’s **primary** `employee_bank` when present. `net_salary` is the transfer amount; not
/// a bank NEFT/RTGS file format from any one bank—generic prep for upload / ops.
pub async fn payroll_bank_transfer_csv(
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
        "employee_code,employee_name,beneficiary_name,bank_name,account_number,ifsc_code,account_type,currency,amount,period_month,period_year,payroll_cycle_name,bank_status,payslip_id\n",
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

    let bank_rows = employee_bank::Entity::find()
        .filter(employee_bank::Column::TenantId.eq(tenant_id))
        .filter(employee_bank::Column::EmployeeId.is_in(emp_ids))
        .filter(employee_bank::Column::IsPrimary.eq(true))
        .all(db)
        .await
        .map_err(KabiPayError::from)?;
    let mut bank_by_emp: HashMap<Uuid, employee_bank::Model> = HashMap::new();
    for b in bank_rows {
        bank_by_emp.entry(b.employee_id).or_insert(b);
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
        let (bank_status, bname, acc, ifsc, atype) = if let Some(b) = bank_by_emp.get(&p.employee_id) {
            (
                "OK",
                b.bank_name.as_str(),
                b.account_number.as_str(),
                b.ifsc_code.as_str(),
                b.account_type.as_deref().unwrap_or(""),
            )
        } else {
            ("MISSING_BANK", "", "", "", "")
        };
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            csv_cell(code),
            csv_cell(&name),
            csv_cell(&name),
            csv_cell(bname),
            csv_cell(acc),
            csv_cell(ifsc),
            csv_cell(atype),
            csv_cell("INR"),
            dec_cell(p.net_salary),
            month,
            year,
            csv_cell(cycle_name),
            bank_status,
            csv_cell(&p.id.to_string()),
        ));
    }

    Ok(out)
}

/// Latest `employment_history.salary` for payroll gross (v1 pay run).
async fn latest_employment_salary<C: ConnectionTrait + Send + Sync>(
    db: &C,
    tenant_id: Uuid,
    employee_id: Uuid,
) -> KabiPayResult<Option<Decimal>> {
    let row = employment_history::Entity::find()
        .filter(employment_history::Column::TenantId.eq(tenant_id))
        .filter(employment_history::Column::EmployeeId.eq(employee_id))
        .filter(employment_history::Column::IsDeleted.eq(false))
        .order_by_desc(employment_history::Column::EffectiveFrom)
        .one(db)
        .await
        .map_err(KabiPayError::from)?;
    Ok(row.and_then(|r| r.salary))
}

/// Resolve the salary component used for the gross line (prefer `BASIC`, else first active EARNING).
async fn resolve_default_earning_component(
    db: &DatabaseConnection,
    tenant_id: Uuid,
) -> KabiPayResult<salary_component::Model> {
    if let Some(basic) = salary_component::Entity::find()
        .filter(salary_component::Column::TenantId.eq(tenant_id))
        .filter(salary_component::Column::Code.eq("BASIC"))
        .filter(salary_component::Column::IsActive.eq(true))
        .one(db)
        .await
        .map_err(KabiPayError::from)?
    {
        return Ok(basic);
    }
    let rows = list_components(db, tenant_id, true, 50).await?;
    rows
        .into_iter()
        .find(|c| c.r#type.eq_ignore_ascii_case("EARNING"))
        .ok_or_else(|| {
            KabiPayError::Validation(
                "no active EARNING salary component — seed salary_component (e.g. BASIC) first"
                    .into(),
            )
        })
}

/// Active `EARNING` `salary_component` with code `ARREAR` (payout of pending accruals).
async fn resolve_arrear_salary_component<C: ConnectionTrait + Send + Sync>(
    db: &C,
    tenant_id: Uuid,
) -> KabiPayResult<salary_component::Model> {
    if let Some(c) = salary_component::Entity::find()
        .filter(salary_component::Column::TenantId.eq(tenant_id))
        .filter(salary_component::Column::Code.eq("ARREAR"))
        .filter(salary_component::Column::IsActive.eq(true))
        .one(db)
        .await
        .map_err(KabiPayError::from)?
    {
        if !c.r#type.eq_ignore_ascii_case("EARNING") {
            return Err(KabiPayError::Validation(
                "salary component ARREAR must have type EARNING".into(),
            ));
        }
        return Ok(c);
    }
    Err(KabiPayError::Validation(
        "arrear payout lines require an active EARNING `salary_component` with code ARREAR"
            .into(),
    ))
}

/// Latest TDS to withhold (per month) for each employee from `tax_computation` for the given India FY
/// (when not null). If multiple rows, the most recently `computed_at` row wins.
async fn tds_by_employee_fy(
    db: &impl ConnectionTrait,
    tenant_id: Uuid,
    employee_ids: &[Uuid],
    fy: i32,
) -> KabiPayResult<HashMap<Uuid, Decimal>> {
    if employee_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = tax_computation::Entity::find()
        .filter(tax_computation::Column::TenantId.eq(tenant_id))
        .filter(tax_computation::Column::EmployeeId.is_in(employee_ids.to_vec()))
        .filter(tax_computation::Column::FiscalYear.eq(fy))
        .all(db)
        .await
        .map_err(KabiPayError::from)?;
    let mut best: HashMap<Uuid, tax_computation::Model> = HashMap::new();
    for r in rows {
        best.entry(r.employee_id)
            .and_modify(|p| {
                if r.computed_at > p.computed_at {
                    *p = r.clone();
                }
            })
            .or_insert(r);
    }
    Ok(best
        .into_iter()
        .filter_map(|(eid, v)| v.tds_per_month.map(|d| (eid, d)))
        .collect())
}

/// v2 pay run: for each ACTIVE employee without a payslip, insert payslip (BASIC = employment salary,
/// optional `ARREAR` line(s)), India statutory stub (EPF, ESI, PT, TDS from `tax_computation`), mark
/// cycle `PROCESSED`. `DRAFT` only.
pub async fn run_payroll_for_cycle(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    cycle_id: Uuid,
    processed_by: Uuid,
) -> KabiPayResult<payroll_cycle::Model> {
    let cycle_row = payroll_cycle::Entity::find()
        .filter(payroll_cycle::Column::Id.eq(cycle_id))
        .filter(payroll_cycle::Column::TenantId.eq(tenant_id))
        .one(db)
        .await
        .map_err(KabiPayError::from)?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "payroll_cycle",
            id: cycle_id.to_string(),
        })?;

    if cycle_row.status.to_ascii_uppercase() != "DRAFT" {
        return Err(KabiPayError::Validation(format!(
            "payroll cycle must be DRAFT to run (current status: {})",
            cycle_row.status
        )));
    }

    let basic_comp = resolve_default_earning_component(db, tenant_id).await?;

    let txn = db.begin().await.map_err(KabiPayError::from)?;

    let existing_slips = payslip::Entity::find()
        .filter(payslip::Column::TenantId.eq(tenant_id))
        .filter(payslip::Column::PayrollCycleId.eq(cycle_id))
        .all(&txn)
        .await
        .map_err(KabiPayError::from)?;
    let mut have: HashSet<Uuid> = existing_slips.iter().map(|p| p.employee_id).collect();

    let employees = employee::Entity::find()
        .filter(employee::Column::TenantId.eq(tenant_id))
        .filter(employee::Column::IsDeleted.eq(false))
        .filter(employee::Column::Status.eq("ACTIVE"))
        .all(&txn)
        .await
        .map_err(KabiPayError::from)?;

    let fy = statutory_india::india_fy_start_year(cycle_row.month, cycle_row.year);
    let emp_id_list: Vec<Uuid> = employees.iter().map(|e| e.id).collect();
    let tds_map = tds_by_employee_fy(&txn, tenant_id, &emp_id_list, fy).await?;

    let now = Utc::now();
    for emp in employees {
        if have.contains(&emp.id) {
            continue;
        }
        let base = latest_employment_salary(&txn, tenant_id, emp.id)
            .await?
            .unwrap_or(Decimal::ZERO);
        let pending = arrear_service::list_pending_by_employee(&txn, tenant_id, emp.id).await?;
        let arrear_sum: Decimal = pending.iter().map(|a| a.amount).sum();
        if base <= Decimal::ZERO && arrear_sum <= Decimal::ZERO {
            continue;
        }
        let gross = (base + arrear_sum).round_dp(2);
        let tds_m = tds_map.get(&emp.id).map(|d| d.round_dp(2));
        let (stat, tds) = statutory_india::compute(gross, tds_m);
        let total_ded = statutory_india::employee_deduction_total(&stat, tds);
        let net = (gross - total_ded).round_dp(2);

        let pid = Uuid::new_v4();

        payslip::ActiveModel {
            id: Set(pid),
            tenant_id: Set(tenant_id),
            employee_id: Set(emp.id),
            payroll_cycle_id: Set(cycle_id),
            gross_salary: Set(gross),
            total_deductions: Set(total_ded),
            net_salary: Set(net),
            pf_employee: Set(Some(stat.pf_employee)),
            pf_employer: Set(Some(stat.pf_employer)),
            esi_employee: Set(Some(stat.esi_employee)),
            esi_employer: Set(Some(stat.esi_employer)),
            tds_amount: Set(Some(tds)),
            professional_tax: Set(Some(stat.professional_tax)),
            uan_number: Set(emp.uan_number.clone()),
            esic_number: Set(emp.esic_number.clone()),
            status: Set("GENERATED".to_string()),
            generated_at: Set(now),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&txn)
        .await
        .map_err(KabiPayError::from)?;

        if base > Decimal::ZERO {
            let line_id = Uuid::new_v4();
            payslip_component::ActiveModel {
                id: Set(line_id),
                tenant_id: Set(tenant_id),
                payslip_id: Set(pid),
                salary_component_id: Set(basic_comp.id),
                amount: Set(base),
                component_type: Set(Some(basic_comp.r#type.clone())),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(&txn)
            .await
            .map_err(KabiPayError::from)?;
        }
        if arrear_sum > Decimal::ZERO {
            let ac = resolve_arrear_salary_component(&txn, tenant_id).await?;
            let line_id = Uuid::new_v4();
            payslip_component::ActiveModel {
                id: Set(line_id),
                tenant_id: Set(tenant_id),
                payslip_id: Set(pid),
                salary_component_id: Set(ac.id),
                amount: Set(arrear_sum),
                component_type: Set(Some(ac.r#type.clone())),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(&txn)
            .await
            .map_err(KabiPayError::from)?;
            let a_ids: Vec<Uuid> = pending.iter().map(|a| a.id).collect();
            arrear_service::mark_applied(&txn, tenant_id, &a_ids, cycle_id).await?;
        }

        have.insert(emp.id);
    }

    let mut cycle_am: payroll_cycle::ActiveModel = cycle_row.into();
    cycle_am.status = Set("PROCESSED".to_string());
    cycle_am.processed_at = Set(Some(now));
    cycle_am.processed_by = Set(Some(processed_by));
    cycle_am.updated_at = Set(now);
    let updated = cycle_am.update(&txn).await.map_err(KabiPayError::from)?;

    txn.commit().await.map_err(KabiPayError::from)?;
    Ok(updated)
}
