//! Tenant-scoped SeaORM queries for benefits.

use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0014_benefits::{benefit_plan, benefit_type};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

pub async fn list_types(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<benefit_type::Model>> {
    let limit = limit.clamp(1, 100);
    benefit_type::Entity::find()
        .filter(benefit_type::Column::TenantId.eq(tenant_id))
        .order_by_asc(benefit_type::Column::Code)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_plans(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    active_only: bool,
    limit: u64,
) -> KabiPayResult<Vec<benefit_plan::Model>> {
    let limit = limit.clamp(1, 100);
    let mut q = benefit_plan::Entity::find()
        .filter(benefit_plan::Column::TenantId.eq(tenant_id));
    if active_only {
        q = q.filter(benefit_plan::Column::IsActive.eq(true));
    }
    q.order_by_asc(benefit_plan::Column::Name)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}
