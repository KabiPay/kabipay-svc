//! Root query resolvers for kabipay-grievance.

use async_graphql::{Context, Object, Result};
use kabipay_common::{
    subgraph::{
        require_client_claims, require_tenant_id, resolve_client_employee_id, tenant_db,
    },
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
        let claims = require_client_claims(ctx)?;
        if !claims.can_use_grievance_self_service() && !claims.can_manage_grievance_tenant_cases() {
            return Err(
                KabiPayError::Forbidden("grievance:self or grievance:manage permission required".into())
                    .into_graphql(),
            );
        }
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = grievance_service::list_categories(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(GrievanceCategoryDto::from).collect())
    }

    /// `grievance:manage` sees tenant-wide cases; others see **their own** cases only.
    async fn grievance_cases(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<GrievanceCaseDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let claims = require_client_claims(ctx)?;
        if !claims.can_use_grievance_self_service() && !claims.can_manage_grievance_tenant_cases() {
            return Err(
                KabiPayError::Forbidden("grievance:self or grievance:manage permission required".into())
                    .into_graphql(),
            );
        }
        let filter = if claims.can_manage_grievance_tenant_cases() {
            None
        } else {
            Some(
                resolve_client_employee_id(ctx, &db, tenant_id)
                    .await
                    .map_err(KabiPayError::into_graphql)?,
            )
        };
        let rows = grievance_service::list_cases(&db, tenant_id, limit, filter)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(GrievanceCaseDto::from).collect())
    }
}
