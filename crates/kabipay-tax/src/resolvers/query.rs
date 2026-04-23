//! Root query resolvers for kabipay-tax.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    subgraph::{require_tenant_id, resolve_client_employee_id, tenant_db},
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::types::{TaxComputationDto, TaxConfigurationVersionDto, TaxSlabDto};
use crate::services::tax_service;

fn parse_uuid(id: &ID, field: &'static str) -> Result<Uuid> {
    Uuid::parse_str(id.as_str())
        .map_err(|e| KabiPayError::Validation(format!("invalid {field}: {e}")).into_graphql())
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn tax_health(&self) -> &'static str {
        "ok"
    }

    /// Tax configuration versions configured for this tenant.
    async fn tax_configurations(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = true)] active_only: bool,
        #[graphql(default = 20)] limit: u64,
    ) -> Result<Vec<TaxConfigurationVersionDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = tax_service::list_configurations(&db, tenant_id, active_only, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows
            .into_iter()
            .map(TaxConfigurationVersionDto::from)
            .collect())
    }

    /// Tax slabs for this tenant (filter by fiscal_year server-side later).
    async fn tax_slabs(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<TaxSlabDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = tax_service::list_slabs(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(TaxSlabDto::from).collect())
    }

    /// Stored per-employee tax computation / declaration rows for a fiscal period.
    /// Omit `employeeId` to use the signed-in user’s employee record.
    async fn tax_computations(
        &self,
        ctx: &Context<'_>,
        employee_id: Option<ID>,
        #[graphql(default = 20)] limit: u64,
    ) -> Result<Vec<TaxComputationDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let emp = if let Some(id) = &employee_id {
            parse_uuid(id, "employeeId")?
        } else {
            resolve_client_employee_id(ctx, &db, tenant_id)
                .await
                .map_err(KabiPayError::into_graphql)?
        };
        let rows = tax_service::list_computations(&db, tenant_id, emp, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(TaxComputationDto::from).collect())
    }
}
