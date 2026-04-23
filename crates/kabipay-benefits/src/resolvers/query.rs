//! Root query resolvers for kabipay-benefits.

use async_graphql::{Context, Object, Result};
use kabipay_common::{
    subgraph::{require_tenant_id, tenant_db},
    KabiPayError,
};

use crate::resolvers::types::{BenefitPlanDto, BenefitTypeDto};
use crate::services::benefits_service;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn benefits_health(&self) -> &'static str {
        "ok"
    }

    async fn benefit_types(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<BenefitTypeDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = benefits_service::list_types(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(BenefitTypeDto::from).collect())
    }

    async fn benefit_plans(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = true)] active_only: bool,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<BenefitPlanDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = benefits_service::list_plans(&db, tenant_id, active_only, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(BenefitPlanDto::from).collect())
    }
}
