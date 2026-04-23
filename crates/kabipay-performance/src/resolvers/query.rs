//! Root query resolvers for kabipay-performance.

use async_graphql::{Context, Object, Result};
use kabipay_common::{
    subgraph::{require_tenant_id, tenant_db},
    KabiPayError,
};

use crate::resolvers::types::{GoalDto, ReviewCycleDto};
use crate::services::performance_service;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn performance_health(&self) -> &'static str {
        "ok"
    }

    async fn review_cycles(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 20)] limit: u64,
    ) -> Result<Vec<ReviewCycleDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = performance_service::list_cycles(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(ReviewCycleDto::from).collect())
    }

    async fn goals(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<GoalDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = performance_service::list_goals(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(GoalDto::from).collect())
    }
}
