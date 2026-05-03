//! Root query resolvers for kabipay-notification.

use async_graphql::{Context, Object, Result};
use kabipay_common::{
    subgraph::{
        require_client_claims, require_tenant_id, tenant_db, try_client_employee_dept_and_location,
    },
    KabiPayError,
};

use crate::resolvers::types::{AnnouncementDto, NotificationDto, NotificationPreferencesGql};
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
        let claims = require_client_claims(ctx)?;
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let bypass = claims.can_manage_notifications();
        let (viewer_dept, viewer_loc) = if bypass {
            (None, None)
        } else {
            match try_client_employee_dept_and_location(&db, tenant_id, claims).await {
                Ok(Some((d, l))) => (d, l),
                Ok(None) => (None, None),
                Err(e) => return Err(e.into_graphql()),
            }
        };
        let prefs = crate::services::notification_preference::load_notification_prefs(
            &db,
            tenant_id,
            claims.sub,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        if !bypass && !prefs.announcements_enabled {
            return Ok(vec![]);
        }
        let rows = notification_service::list_announcements_visible(
            &db,
            tenant_id,
            limit,
            bypass,
            viewer_dept,
            viewer_loc,
            &claims.roles,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(AnnouncementDto::from).collect())
    }

    /// Admin / HR: all recent announcements including scheduled or expired (for management UI).
    async fn admin_announcements(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<AnnouncementDto>> {
        let claims = require_client_claims(ctx)?;
        if !claims.can_manage_notifications() {
            return Err(
                KabiPayError::Forbidden("notification:manage or equivalent role required".into())
                    .into_graphql(),
            );
        }
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = notification_service::list_announcements_admin(&db, tenant_id, limit)
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
        let claims = require_client_claims(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows =
            notification_service::list_notifications_for_user(&db, tenant_id, claims.sub, limit)
                .await
                .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(NotificationDto::from).collect())
    }

    /// Admin / HR: recent in-app notifications tenant-wide (for support / auditing).
    async fn admin_notifications(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<NotificationDto>> {
        let claims = require_client_claims(ctx)?;
        if !claims.can_manage_notifications() {
            return Err(
                KabiPayError::Forbidden("notification:manage or equivalent role required".into())
                    .into_graphql(),
            );
        }
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = notification_service::list_notifications(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(NotificationDto::from).collect())
    }

    async fn unread_notification_count(&self, ctx: &Context<'_>) -> Result<u64> {
        let tenant_id = require_tenant_id(ctx)?;
        let claims = require_client_claims(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        notification_service::count_unread_for_user(&db, tenant_id, claims.sub)
            .await
            .map_err(KabiPayError::into_graphql)
    }

    /// Current user’s in-app visibility preferences (announcement bulletin + per-topic mutes).
    async fn my_notification_preferences(&self, ctx: &Context<'_>) -> Result<NotificationPreferencesGql> {
        let tenant_id = require_tenant_id(ctx)?;
        let claims = require_client_claims(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let p = crate::services::notification_preference::load_notification_prefs(
            &db,
            tenant_id,
            claims.sub,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(NotificationPreferencesGql::from_prefs(p))
    }
}
