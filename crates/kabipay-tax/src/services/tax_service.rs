//! Tenant-scoped SeaORM queries and commands for tax configuration, slabs, and employee computations.

use chrono::Utc;
use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0007_employee_core::employee;
use kabipay_db_entities::tenant::d0013_tax_statutory::{
    tax_computation, tax_configuration_version, tax_slab,
};
use kabipay_db_entities::tenant::d0027_communication_audit::notification;
use kabipay_db_entities::tenant::d0031_tax_proof::tax_proof_line;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use std::str::FromStr;
use uuid::Uuid;

pub async fn list_configurations(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    active_only: bool,
    limit: u64,
) -> KabiPayResult<Vec<tax_configuration_version::Model>> {
    let limit = limit.clamp(1, 40);
    let mut q = tax_configuration_version::Entity::find()
        .filter(tax_configuration_version::Column::TenantId.eq(tenant_id));
    if active_only {
        q = q.filter(tax_configuration_version::Column::IsActive.eq(true));
    }
    q.order_by_desc(tax_configuration_version::Column::FiscalYear)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_slabs(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<tax_slab::Model>> {
    let limit = limit.clamp(1, 200);
    tax_slab::Entity::find()
        .filter(tax_slab::Column::TenantId.eq(tenant_id))
        .order_by_asc(tax_slab::Column::IncomeFrom)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_computations(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<tax_computation::Model>> {
    let limit = limit.clamp(1, 100);
    tax_computation::Entity::find()
        .filter(tax_computation::Column::TenantId.eq(tenant_id))
        .filter(tax_computation::Column::EmployeeId.eq(employee_id))
        .order_by_desc(tax_computation::Column::FiscalYear)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

/// Create or update the row keyed by (tenant, employee, tax config version, fiscal year).
pub async fn upsert_tax_computation(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    tax_config_version_id: Uuid,
    fiscal_year: i32,
    tax_regime_chosen: Option<String>,
    gross_income: Option<Decimal>,
    total_deductions: Option<Decimal>,
    taxable_income: Option<Decimal>,
    final_tax: Option<Decimal>,
    tds_per_month: Option<Decimal>,
) -> KabiPayResult<tax_computation::Model> {
    let _ver = tax_configuration_version::Entity::find()
        .filter(tax_configuration_version::Column::Id.eq(tax_config_version_id))
        .filter(tax_configuration_version::Column::TenantId.eq(tenant_id))
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "tax_configuration_version",
            id: tax_config_version_id.to_string(),
        })?;
    if _ver.fiscal_year != fiscal_year {
        return Err(KabiPayError::Validation(
            "fiscalYear does not match the selected tax configuration version".into(),
        ));
    }
    let existing = tax_computation::Entity::find()
        .filter(tax_computation::Column::TenantId.eq(tenant_id))
        .filter(tax_computation::Column::EmployeeId.eq(employee_id))
        .filter(tax_computation::Column::TaxConfigVersionId.eq(tax_config_version_id))
        .filter(tax_computation::Column::FiscalYear.eq(fiscal_year))
        .one(db)
        .await?;
    let now = Utc::now();
    if let Some(row) = existing {
        let id = row.id;
        let mut am: tax_computation::ActiveModel = row.into();
        am.tax_regime_chosen = Set(tax_regime_chosen);
        am.gross_income = Set(gross_income);
        am.total_deductions = Set(total_deductions);
        am.taxable_income = Set(taxable_income);
        am.final_tax = Set(final_tax);
        am.tds_per_month = Set(tds_per_month);
        am.computed_at = Set(now);
        am.update(db).await?;
        tax_computation::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| KabiPayError::Internal("updated tax_computation not found".into()))
    } else {
        let id = Uuid::new_v4();
        let am = tax_computation::ActiveModel {
            id: Set(id),
            tenant_id: Set(tenant_id),
            employee_id: Set(employee_id),
            tax_config_version_id: Set(tax_config_version_id),
            fiscal_year: Set(fiscal_year),
            tax_regime_chosen: Set(tax_regime_chosen),
            gross_income: Set(gross_income),
            total_deductions: Set(total_deductions),
            taxable_income: Set(taxable_income),
            final_tax: Set(final_tax),
            tds_per_month: Set(tds_per_month),
            computed_at: Set(now),
        };
        am.insert(db).await?;
        tax_computation::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| KabiPayError::Internal("inserted tax_computation not found".into()))
    }
}

pub fn opt_decimal(s: &Option<String>) -> KabiPayResult<Option<Decimal>> {
    match s {
        None => Ok(None),
        Some(t) if t.trim().is_empty() => Ok(None),
        Some(t) => Decimal::from_str(t.trim())
            .map(Some)
            .map_err(|_| KabiPayError::Validation("invalid decimal string in tax fields".into())),
    }
}

const PROOF_PENDING: &str = "PENDING";
const PROOF_APPROVED: &str = "APPROVED";
const PROOF_REJECTED: &str = "REJECTED";

pub async fn list_tax_proof_lines(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    tax_config_version_id: Option<Uuid>,
    fiscal_year: Option<i32>,
) -> KabiPayResult<Vec<tax_proof_line::Model>> {
    let mut q = tax_proof_line::Entity::find()
        .filter(tax_proof_line::Column::TenantId.eq(tenant_id))
        .filter(tax_proof_line::Column::EmployeeId.eq(employee_id));
    if let Some(v) = tax_config_version_id {
        q = q.filter(tax_proof_line::Column::TaxConfigVersionId.eq(v));
    }
    if let Some(y) = fiscal_year {
        q = q.filter(tax_proof_line::Column::FiscalYear.eq(y));
    }
    q.order_by_asc(tax_proof_line::Column::SectionCode)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

/// Employee submits or updates a proof line (e.g. 80C, HRA) — goes to **PENDING** until approved.
/// Only **APPROVED** lines roll into `tax_computation.total_deductions` (see
/// `recompute_total_deductions_from_approved_proofs`).
pub async fn submit_tax_proof_line(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    tax_config_version_id: Uuid,
    fiscal_year: i32,
    section_code: String,
    declared_amount: Decimal,
    actual_amount: Decimal,
    file_storage_id: Option<Uuid>,
) -> KabiPayResult<tax_proof_line::Model> {
    if declared_amount < Decimal::ZERO || actual_amount < Decimal::ZERO {
        return Err(KabiPayError::Validation(
            "declared and actual amounts must be non-negative".into(),
        ));
    }
    let sc = section_code.trim().to_string();
    if sc.is_empty() {
        return Err(KabiPayError::Validation("sectionCode is required".into()));
    }
    let _ver = tax_configuration_version::Entity::find()
        .filter(tax_configuration_version::Column::Id.eq(tax_config_version_id))
        .filter(tax_configuration_version::Column::TenantId.eq(tenant_id))
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "tax_configuration_version",
            id: tax_config_version_id.to_string(),
        })?;
    if _ver.fiscal_year != fiscal_year {
        return Err(KabiPayError::Validation(
            "fiscalYear does not match the tax configuration version".into(),
        ));
    }

    let existing = tax_proof_line::Entity::find()
        .filter(tax_proof_line::Column::TenantId.eq(tenant_id))
        .filter(tax_proof_line::Column::EmployeeId.eq(employee_id))
        .filter(tax_proof_line::Column::TaxConfigVersionId.eq(tax_config_version_id))
        .filter(tax_proof_line::Column::SectionCode.eq(&sc))
        .one(db)
        .await?;

    let now = Utc::now();
    let out = if let Some(row) = existing {
        let id = row.id;
        let mut am: tax_proof_line::ActiveModel = row.into();
        am.declared_amount = Set(declared_amount);
        am.actual_amount = Set(actual_amount);
        am.file_storage_id = Set(file_storage_id);
        am.status = Set(PROOF_PENDING.into());
        am.rejection_reason = Set(None);
        am.approved_by = Set(None);
        am.submitted_at = Set(now);
        am.fiscal_year = Set(fiscal_year);
        am.update(db).await?;
        tax_proof_line::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| KabiPayError::Internal("updated tax_proof_line not found".into()))?
    } else {
        let id = Uuid::new_v4();
        let am = tax_proof_line::ActiveModel {
            id: Set(id),
            tenant_id: Set(tenant_id),
            employee_id: Set(employee_id),
            tax_config_version_id: Set(tax_config_version_id),
            fiscal_year: Set(fiscal_year),
            section_code: Set(sc),
            declared_amount: Set(declared_amount),
            actual_amount: Set(actual_amount),
            file_storage_id: Set(file_storage_id),
            status: Set(PROOF_PENDING.into()),
            rejection_reason: Set(None),
            approved_by: Set(None),
            submitted_at: Set(now),
            created_at: Set(now),
            updated_at: Set(now),
        };
        am.insert(db).await?;
        tax_proof_line::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| KabiPayError::Internal("inserted tax_proof_line not found".into()))?
    };

    recompute_total_deductions_from_approved_proofs(
        db,
        tenant_id,
        employee_id,
        tax_config_version_id,
        fiscal_year,
    )
    .await?;

    Ok(out)
}

async fn load_pending_tax_proof(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    line_id: Uuid,
) -> KabiPayResult<tax_proof_line::Model> {
    let m = tax_proof_line::Entity::find()
        .filter(tax_proof_line::Column::Id.eq(line_id))
        .filter(tax_proof_line::Column::TenantId.eq(tenant_id))
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "tax_proof_line",
            id: line_id.to_string(),
        })?;
    if m.status != PROOF_PENDING {
        return Err(KabiPayError::Validation(
            "only PENDING tax proof lines can be approved or rejected".into(),
        ));
    }
    Ok(m)
}

pub async fn approve_tax_proof_line(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    line_id: Uuid,
    approver_user_id: Uuid,
) -> KabiPayResult<tax_proof_line::Model> {
    let model = load_pending_tax_proof(db, tenant_id, line_id).await?;
    let tid = model.tax_config_version_id;
    let eid = model.employee_id;
    let fy = model.fiscal_year;
    let mut am: tax_proof_line::ActiveModel = model.into();
    am.status = Set(PROOF_APPROVED.into());
    am.rejection_reason = Set(None);
    am.approved_by = Set(Some(approver_user_id));
    am.update(db).await?;
    let out = tax_proof_line::Entity::find_by_id(line_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("updated tax_proof_line not found".into()))?;
    recompute_total_deductions_from_approved_proofs(db, tenant_id, eid, tid, fy).await?;
    tax_proof_notify_employee(
        db,
        tenant_id,
        eid,
        "Tax proof approved",
        &format!(
            "Your {} deduction proof (FY {}) was approved and counts toward year-end tax.",
            out.section_code, out.fiscal_year
        ),
    )
    .await;
    Ok(out)
}

pub async fn reject_tax_proof_line(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    line_id: Uuid,
    rejection_reason: Option<String>,
) -> KabiPayResult<tax_proof_line::Model> {
    let model = load_pending_tax_proof(db, tenant_id, line_id).await?;
    let tid = model.tax_config_version_id;
    let eid = model.employee_id;
    let fy = model.fiscal_year;
    let mut am: tax_proof_line::ActiveModel = model.into();
    am.status = Set(PROOF_REJECTED.into());
    am.rejection_reason = Set(rejection_reason);
    am.approved_by = Set(None);
    am.update(db).await?;
    let out = tax_proof_line::Entity::find_by_id(line_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("updated tax_proof_line not found".into()))?;
    recompute_total_deductions_from_approved_proofs(db, tenant_id, eid, tid, fy).await?;
    let msg = match &out.rejection_reason {
        Some(s) if !s.is_empty() => format!(
            "Your {} proof (FY {}) was rejected. Reason: {s}",
            out.section_code, out.fiscal_year
        ),
        _ => format!(
            "Your {} proof (FY {}) was rejected. It will not count toward tax deductions until resubmitted and approved.",
            out.section_code, out.fiscal_year
        ),
    };
    tax_proof_notify_employee(db, tenant_id, eid, "Tax proof rejected", &msg).await;
    Ok(out)
}

/// Sums `actual_amount` for **APPROVED** lines and writes the result to
/// `tax_computation.total_deductions` for the same employee / config / fiscal year.
/// Year-end and payroll logic should use that column (not unapproved `actual_amount` values).
pub async fn recompute_total_deductions_from_approved_proofs(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    tax_config_version_id: Uuid,
    fiscal_year: i32,
) -> KabiPayResult<()> {
    let lines = tax_proof_line::Entity::find()
        .filter(tax_proof_line::Column::TenantId.eq(tenant_id))
        .filter(tax_proof_line::Column::EmployeeId.eq(employee_id))
        .filter(tax_proof_line::Column::TaxConfigVersionId.eq(tax_config_version_id))
        .filter(tax_proof_line::Column::FiscalYear.eq(fiscal_year))
        .filter(tax_proof_line::Column::Status.eq(PROOF_APPROVED))
        .all(db)
        .await?;
    let sum: Decimal = lines
        .iter()
        .fold(Decimal::ZERO, |acc, l| acc + l.actual_amount);
    let now = Utc::now();
    let existing = tax_computation::Entity::find()
        .filter(tax_computation::Column::TenantId.eq(tenant_id))
        .filter(tax_computation::Column::EmployeeId.eq(employee_id))
        .filter(tax_computation::Column::TaxConfigVersionId.eq(tax_config_version_id))
        .filter(tax_computation::Column::FiscalYear.eq(fiscal_year))
        .one(db)
        .await?;
    if let Some(row) = existing {
        let mut am: tax_computation::ActiveModel = row.into();
        am.total_deductions = Set(Some(sum));
        am.computed_at = Set(now);
        am.update(db).await?;
    } else {
        let id = Uuid::new_v4();
        let am = tax_computation::ActiveModel {
            id: Set(id),
            tenant_id: Set(tenant_id),
            employee_id: Set(employee_id),
            tax_config_version_id: Set(tax_config_version_id),
            fiscal_year: Set(fiscal_year),
            tax_regime_chosen: Set(None),
            gross_income: Set(None),
            total_deductions: Set(Some(sum)),
            taxable_income: Set(None),
            final_tax: Set(None),
            tds_per_month: Set(None),
            computed_at: Set(now),
        };
        am.insert(db).await?;
    }
    Ok(())
}

async fn tax_proof_notify_employee(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    title: &str,
    message: &str,
) {
    let user_id: Option<Uuid> = match employee::Entity::find_by_id(employee_id)
        .filter(employee::Column::TenantId.eq(tenant_id))
        .filter(employee::Column::IsDeleted.eq(false))
        .one(db)
        .await
    {
        Ok(Some(emp)) => emp.user_id,
        _ => None,
    };
    let Some(user_id) = user_id else {
        return;
    };
    let now = Utc::now();
    let am = notification::ActiveModel {
        id: Set(Uuid::new_v4()),
        tenant_id: Set(tenant_id),
        user_id: Set(user_id),
        r#type: Set(Some("TAX".into())),
        title: Set(Some(title.into())),
        message: Set(Some(message.into())),
        action_url: Set(None),
        is_read: Set(false),
        read_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    if let Err(e) = am.insert(db).await {
        tracing::warn!(error = %e, "insert notification (tax proof) failed");
    }
}
