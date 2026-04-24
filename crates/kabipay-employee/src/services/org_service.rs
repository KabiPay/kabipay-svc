//! Tenant-scoped org hierarchy reads (departments, designations).

use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0006_org_hierarchy::{department, designation};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

pub async fn list_departments(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<department::Model>> {
    let limit = limit.clamp(1, 200);
    department::Entity::find()
        .filter(department::Column::TenantId.eq(tenant_id))
        .filter(department::Column::IsDeleted.eq(false))
        .order_by_asc(department::Column::Name)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_designations(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<designation::Model>> {
    let limit = limit.clamp(1, 200);
    designation::Entity::find()
        .filter(designation::Column::TenantId.eq(tenant_id))
        .filter(designation::Column::IsDeleted.eq(false))
        .order_by_asc(designation::Column::Title)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}
