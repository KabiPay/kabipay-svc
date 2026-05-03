//! Root query resolvers for kabipay-succession.

use async_graphql::{Context, Object, Result};
use kabipay_common::{
    subgraph::{require_client_claims, require_tenant_id, tenant_db},
    KabiPayError,
};

use crate::resolvers::types::{CompetencyDto, TalentPoolDto};
use crate::services::succession_service;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn succession_health(&self) -> &'static str {
        "ok"
    }

    async fn competencies(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<CompetencyDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = succession_service::list_competencies(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(CompetencyDto::from).collect())
    }

    async fn talent_pools(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<TalentPoolDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let claims = require_client_claims(ctx)?;
        if !claims.can_manage_succession_planning() {
            return Err(
                KabiPayError::Forbidden("succession:manage permission required".into()).into_graphql(),
            );
        }
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = succession_service::list_pools(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(TalentPoolDto::from).collect())
    }
}
