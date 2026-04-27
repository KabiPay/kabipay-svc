//! Root query resolvers for kabipay-workflow.

use async_graphql::{Context, ID, Object, Result};
use kabipay_common::{
    subgraph::{require_tenant_id, tenant_db},
    KabiPayError,
};

use uuid::Uuid;

use crate::resolvers::types::{WorkflowDto, WorkflowInstanceDto, WorkflowWithStepsDto, WorkflowStepDto};
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

    /// All active workflow definitions, each with ordered steps (read-only “designer” data).
    async fn workflows_with_steps(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<WorkflowWithStepsDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let wfs = workflow_service::list_workflows(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let mut out: Vec<WorkflowWithStepsDto> = Vec::with_capacity(wfs.len());
        for w in wfs {
            let steps = workflow_service::list_workflow_steps(&db, tenant_id, w.id)
                .await
                .map_err(KabiPayError::into_graphql)?
                .into_iter()
                .map(WorkflowStepDto::from)
                .collect();
            out.push(WorkflowWithStepsDto {
                workflow: WorkflowDto::from(w),
                steps,
            });
        }
        Ok(out)
    }

    /// Step list for a single workflow (same ordering as `workflowsWithSteps` per workflow).
    async fn workflow_steps(
        &self,
        ctx: &Context<'_>,
        workflow_id: ID,
    ) -> Result<Vec<WorkflowStepDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let id = Uuid::parse_str(workflow_id.as_str())
            .map_err(|e| {
                KabiPayError::Validation(format!("invalid workflowId: {e}")).into_graphql()
            })?;
        let steps = workflow_service::list_workflow_steps(&db, tenant_id, id)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(steps.into_iter().map(WorkflowStepDto::from).collect())
    }
}
