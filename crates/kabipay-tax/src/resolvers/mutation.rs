//! Write operations for employee tax computations / declarations.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    subgraph::{require_tenant_id, resolve_client_employee_id, tenant_db},
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::types::{TaxComputationDto, UpsertTaxComputationInput};
use crate::services::tax_service;

fn parse_uuid(id: &ID, field: &'static str) -> Result<Uuid> {
    Uuid::parse_str(id.as_str())
        .map_err(|e| KabiPayError::Validation(format!("invalid {field}: {e}")).into_graphql())
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Create or update the `tax_computation` row for this employee, config version, and year.
    async fn upsert_tax_computation(
        &self,
        ctx: &Context<'_>,
        input: UpsertTaxComputationInput,
    ) -> Result<TaxComputationDto> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let employee_id = resolve_client_employee_id(ctx, &db, tenant_id)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let v = parse_uuid(&input.tax_config_version_id, "taxConfigVersionId")?;
        let m = tax_service::upsert_tax_computation(
            &db,
            tenant_id,
            employee_id,
            v,
            input.fiscal_year,
            input.tax_regime_chosen,
            tax_service::opt_decimal(&input.gross_income).map_err(KabiPayError::into_graphql)?,
            tax_service::opt_decimal(&input.total_deductions).map_err(KabiPayError::into_graphql)?,
            tax_service::opt_decimal(&input.taxable_income).map_err(KabiPayError::into_graphql)?,
            tax_service::opt_decimal(&input.final_tax).map_err(KabiPayError::into_graphql)?,
            tax_service::opt_decimal(&input.tds_per_month).map_err(KabiPayError::into_graphql)?,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(TaxComputationDto::from(m))
    }
}
