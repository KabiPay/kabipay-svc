//! Root query resolvers for kabipay-grievance.

use async_graphql::{Context, Object, Result};
use kabipay_common::{
    subgraph::{require_tenant_id, tenant_db},
    KabiPayError,
};

use crate::resolvers::types::{GrievanceCaseDto, GrievanceCategoryDto};
use crate::services::grievance_service;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn grievance_health(&self) -> &'static str {
        "ok"
    }

    async fn grievance_categories(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<GrievanceCategoryDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = grievance_service::list_categories(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(GrievanceCategoryDto::from).collect())
    }

    async fn grievance_cases(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<GrievanceCaseDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = grievance_service::list_cases(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(GrievanceCaseDto::from).collect())
    }
}
