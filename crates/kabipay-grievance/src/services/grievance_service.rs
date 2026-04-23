//! Tenant-scoped SeaORM queries for grievance cases.

use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0023_grievance::{grievance_case, grievance_category};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

pub async fn list_categories(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<grievance_category::Model>> {
    let limit = limit.clamp(1, 200);
    grievance_category::Entity::find()
        .filter(grievance_category::Column::TenantId.eq(tenant_id))
        .order_by_asc(grievance_category::Column::Name)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_cases(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<grievance_case::Model>> {
    let limit = limit.clamp(1, 500);
    grievance_case::Entity::find()
        .filter(grievance_case::Column::TenantId.eq(tenant_id))
        .order_by_desc(grievance_case::Column::FiledAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}
