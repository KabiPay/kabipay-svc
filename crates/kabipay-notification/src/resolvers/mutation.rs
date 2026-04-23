//! Write operations for notifications (read state).

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    subgraph::{require_client_claims, require_tenant_id, tenant_db},
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::types::NotificationDto;
use crate::services::notification_service;

fn parse_uuid(id: &ID, field: &'static str) -> Result<Uuid> {
    Uuid::parse_str(id.as_str())
        .map_err(|e| KabiPayError::Validation(format!("invalid {field}: {e}")).into_graphql())
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Mark one in-app notification as read (must belong to the caller’s `user` id in the JWT).
    async fn mark_notification_read(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> Result<NotificationDto> {
        let tenant_id = require_tenant_id(ctx)?;
        let claims = require_client_claims(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let nid = parse_uuid(&id, "id")?;
        let m = notification_service::mark_read(&db, tenant_id, claims.sub, nid)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(NotificationDto::from(m))
    }

    /// Mark every unread notification for this user as read. Returns how many rows were updated.
    async fn mark_all_notifications_read(&self, ctx: &Context<'_>) -> Result<u64> {
        let tenant_id = require_tenant_id(ctx)?;
        let claims = require_client_claims(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let n = notification_service::mark_all_read(&db, tenant_id, claims.sub)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(n)
    }
}
