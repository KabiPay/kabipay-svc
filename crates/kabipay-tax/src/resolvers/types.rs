//! GraphQL DTOs for kabipay-tax.

use async_graphql::{InputObject, SimpleObject, ID};
use chrono::{DateTime, Utc};
use kabipay_db_entities::tenant::d0013_tax_statutory::{
    tax_computation, tax_configuration_version, tax_slab,
};

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "TaxConfigurationVersion")]
pub struct TaxConfigurationVersionDto {
    pub id: ID,
    pub tenant_id: ID,
    pub fiscal_year: i32,
    pub regime: Option<String>,
    pub country_code: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<tax_configuration_version::Model> for TaxConfigurationVersionDto {
    fn from(m: tax_configuration_version::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            fiscal_year: m.fiscal_year,
            regime: m.regime,
            country_code: m.country_code,
            is_active: m.is_active,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "TaxSlab")]
pub struct TaxSlabDto {
    pub id: ID,
    pub tenant_id: ID,
    pub tax_config_version_id: ID,
    /// Decimal rendered as string for lossless transport.
    pub income_from: String,
    pub income_to: Option<String>,
    pub tax_rate: Option<String>,
    pub surcharge_rate: Option<String>,
    pub cess_rate: Option<String>,
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "TaxComputation")]
pub struct TaxComputationDto {
    pub id: ID,
    pub tenant_id: ID,
    pub employee_id: ID,
    pub tax_config_version_id: ID,
    pub fiscal_year: i32,
    pub tax_regime_chosen: Option<String>,
    pub gross_income: Option<String>,
    pub total_deductions: Option<String>,
    pub taxable_income: Option<String>,
    pub final_tax: Option<String>,
    pub tds_per_month: Option<String>,
    pub computed_at: DateTime<Utc>,
}

impl From<tax_computation::Model> for TaxComputationDto {
    fn from(m: tax_computation::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            employee_id: ID(m.employee_id.to_string()),
            tax_config_version_id: ID(m.tax_config_version_id.to_string()),
            fiscal_year: m.fiscal_year,
            tax_regime_chosen: m.tax_regime_chosen,
            gross_income: m.gross_income.map(|d| d.to_string()),
            total_deductions: m.total_deductions.map(|d| d.to_string()),
            taxable_income: m.taxable_income.map(|d| d.to_string()),
            final_tax: m.final_tax.map(|d| d.to_string()),
            tds_per_month: m.tds_per_month.map(|d| d.to_string()),
            computed_at: m.computed_at,
        }
    }
}

#[derive(InputObject, Clone, Debug)]
pub struct UpsertTaxComputationInput {
    pub tax_config_version_id: ID,
    pub fiscal_year: i32,
    pub tax_regime_chosen: Option<String>,
    pub gross_income: Option<String>,
    pub total_deductions: Option<String>,
    pub taxable_income: Option<String>,
    pub final_tax: Option<String>,
    pub tds_per_month: Option<String>,
}

impl From<tax_slab::Model> for TaxSlabDto {
    fn from(m: tax_slab::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            tax_config_version_id: ID(m.tax_config_version_id.to_string()),
            income_from: m.income_from.to_string(),
            income_to: m.income_to.map(|d| d.to_string()),
            tax_rate: m.tax_rate.map(|d| d.to_string()),
            surcharge_rate: m.surcharge_rate.map(|d| d.to_string()),
            cess_rate: m.cess_rate.map(|d| d.to_string()),
        }
    }
}
