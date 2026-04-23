//! Root query resolvers for kabipay-tenant (ops plane).

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{subgraph::ops_db, KabiPayError};
use uuid::Uuid;

use crate::resolvers::types::{ModuleDto, TenantDto, TenantSubscriptionDto};
use crate::services::tenant_service;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn tenant_health(&self) -> &'static str {
        "ok"
    }

    async fn tenants(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<TenantDto>> {
        let db = ops_db(ctx)?;
        let rows = tenant_service::list_tenants(db, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(TenantDto::from).collect())
    }

    async fn modules(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<ModuleDto>> {
        let db = ops_db(ctx)?;
        let rows = tenant_service::list_modules(db, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(ModuleDto::from).collect())
    }

    async fn tenant_subscriptions(
        &self,
        ctx: &Context<'_>,
        tenant_id: Option<ID>,
        #[graphql(default = 200)] limit: u64,
    ) -> Result<Vec<TenantSubscriptionDto>> {
        let db = ops_db(ctx)?;
        let tenant = match tenant_id {
            None => None,
            Some(id) => Some(Uuid::parse_str(&id.0).map_err(|e| {
                KabiPayError::Validation(format!("tenantId is not a UUID: {e}")).into_graphql()
            })?),
        };
        let rows = tenant_service::list_subscriptions(db, tenant, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows
            .into_iter()
            .map(TenantSubscriptionDto::from)
            .collect())
    }
}
