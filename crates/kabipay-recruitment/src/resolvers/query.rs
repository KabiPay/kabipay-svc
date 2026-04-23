//! Root query resolvers for kabipay-recruitment.

use async_graphql::{Context, Object, Result};
use kabipay_common::{
    subgraph::{require_tenant_id, tenant_db},
    KabiPayError,
};

use crate::resolvers::types::{ApplicationDto, JobPostingDto};
use crate::services::recruitment_service;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn recruitment_health(&self) -> &'static str {
        "ok"
    }

    async fn job_postings(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<JobPostingDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = recruitment_service::list_jobs(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(JobPostingDto::from).collect())
    }

    async fn applications(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<ApplicationDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = recruitment_service::list_applications(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(ApplicationDto::from).collect())
    }
}
