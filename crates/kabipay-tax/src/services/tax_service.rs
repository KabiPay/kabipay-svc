//! Tenant-scoped SeaORM queries and commands for tax configuration, slabs, and employee computations.

use chrono::Utc;
use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0013_tax_statutory::{
    tax_computation, tax_configuration_version, tax_slab,
};
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
