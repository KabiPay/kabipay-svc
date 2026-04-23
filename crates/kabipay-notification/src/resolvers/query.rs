//! Root query resolvers for kabipay-notification.

use async_graphql::{Context, Object, Result};
use kabipay_common::{
    subgraph::{require_tenant_id, tenant_db},
    KabiPayError,
};

use crate::resolvers::types::{AnnouncementDto, NotificationDto};
use crate::services::notification_service;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn notification_health(&self) -> &'static str {
        "ok"
    }

    async fn announcements(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<AnnouncementDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = notification_service::list_announcements(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(AnnouncementDto::from).collect())
    }

    async fn notifications(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<NotificationDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = notification_service::list_notifications(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(NotificationDto::from).collect())
    }
}
