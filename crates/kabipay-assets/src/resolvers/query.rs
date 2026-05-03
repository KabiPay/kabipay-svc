//! Root query resolvers for kabipay-assets.

use async_graphql::{Context, Object, Result};
use kabipay_common::{
    subgraph::{require_client_claims, require_tenant_id, tenant_db},
    KabiPayError,
};

use crate::resolvers::types::{AssetCategoryDto, AssetDto};
use crate::services::asset_service;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn assets_health(&self) -> &'static str {
        "ok"
    }

    async fn asset_categories(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<AssetCategoryDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let claims = require_client_claims(ctx)?;
        if !claims.can_manage_assets_registry() {
            return Err(
                KabiPayError::Forbidden("assets:manage permission required".into()).into_graphql(),
            );
        }
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = asset_service::list_categories(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(AssetCategoryDto::from).collect())
    }

    async fn assets(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<AssetDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let claims = require_client_claims(ctx)?;
        if !claims.can_manage_assets_registry() {
            return Err(
                KabiPayError::Forbidden("assets:manage permission required".into()).into_graphql(),
            );
        }
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = asset_service::list_assets(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(AssetDto::from).collect())
    }
}
