//! Tenant-scoped SeaORM queries and commands for in-app notifications and announcements.

use chrono::Utc;
use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0027_communication_audit::{announcement, notification};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use uuid::Uuid;

pub async fn list_announcements(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<announcement::Model>> {
    let limit = limit.clamp(1, 200);
    announcement::Entity::find()
        .filter(announcement::Column::TenantId.eq(tenant_id))
        .order_by_desc(announcement::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_notifications(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<notification::Model>> {
    let limit = limit.clamp(1, 500);
    notification::Entity::find()
        .filter(notification::Column::TenantId.eq(tenant_id))
        .order_by_desc(notification::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

/// Mark a single row read; returns `NotFound` if wrong id/tenant or not owned by `user_id`.
pub async fn mark_read(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    user_id: Uuid,
    notification_id: Uuid,
) -> KabiPayResult<notification::Model> {
    let row = notification::Entity::find()
        .filter(notification::Column::Id.eq(notification_id))
        .filter(notification::Column::TenantId.eq(tenant_id))
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "notification",
            id: notification_id.to_string(),
        })?;
    if row.user_id != user_id {
        return Err(KabiPayError::Forbidden(
            "notification belongs to another user".into(),
        ));
    }
    if row.is_read {
        return Ok(row);
    }
    let mut am: notification::ActiveModel = row.into();
    am.is_read = Set(true);
    am.read_at = Set(Some(Utc::now()));
    am.updated_at = Set(Utc::now());
    am.update(db).await?;
    notification::Entity::find_by_id(notification_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("notification row missing after update".into()))
}

/// Set `is_read` on all unread rows for a user in a tenant.
pub async fn mark_all_read(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    user_id: Uuid,
) -> KabiPayResult<u64> {
    let rows: Vec<notification::Model> = notification::Entity::find()
        .filter(notification::Column::TenantId.eq(tenant_id))
        .filter(notification::Column::UserId.eq(user_id))
        .filter(notification::Column::IsRead.eq(false))
        .all(db)
        .await?;
    let n = rows.len() as u64;
    for row in rows {
        let mut am: notification::ActiveModel = row.into();
        am.is_read = Set(true);
        am.read_at = Set(Some(Utc::now()));
        am.updated_at = Set(Utc::now());
        am.update(db).await?;
    }
    Ok(n)
}
