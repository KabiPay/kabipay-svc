//! Ops-plane SeaORM queries for operator users and roles.

use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::ops::{operator_role, operator_user};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

pub async fn list_users(
    db: &DatabaseConnection,
    limit: u64,
) -> KabiPayResult<Vec<operator_user::Model>> {
    let limit = limit.clamp(1, 500);
    operator_user::Entity::find()
        .filter(operator_user::Column::IsDeleted.eq(false))
        .order_by_asc(operator_user::Column::FullName)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_roles(
    db: &DatabaseConnection,
    limit: u64,
) -> KabiPayResult<Vec<operator_role::Model>> {
    let limit = limit.clamp(1, 100);
    operator_role::Entity::find()
        .order_by_asc(operator_role::Column::Name)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}
