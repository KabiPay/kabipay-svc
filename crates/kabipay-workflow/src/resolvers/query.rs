//! Root query resolvers for kabipay-workflow.

use async_graphql::{Context, Object, Result};
use kabipay_common::{
    subgraph::{require_tenant_id, tenant_db},
    KabiPayError,
};

use crate::resolvers::types::{WorkflowDto, WorkflowInstanceDto};
use crate::services::workflow_service;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn workflow_health(&self) -> &'static str {
        "ok"
    }

    async fn workflows(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<WorkflowDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = workflow_service::list_workflows(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(WorkflowDto::from).collect())
    }

    async fn workflow_instances(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<WorkflowInstanceDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = workflow_service::list_instances(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(WorkflowInstanceDto::from).collect())
    }
}
