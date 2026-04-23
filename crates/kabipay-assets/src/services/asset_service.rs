//! Tenant-scoped SeaORM queries for assets.

use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0022_assets::{asset, asset_category};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

pub async fn list_categories(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<asset_category::Model>> {
    let limit = limit.clamp(1, 200);
    asset_category::Entity::find()
        .filter(asset_category::Column::TenantId.eq(tenant_id))
        .order_by_asc(asset_category::Column::Name)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_assets(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<asset::Model>> {
    let limit = limit.clamp(1, 500);
    asset::Entity::find()
        .filter(asset::Column::TenantId.eq(tenant_id))
        .order_by_desc(asset::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}
