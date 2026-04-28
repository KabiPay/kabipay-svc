//! Ops-plane SeaORM queries for operator users and roles.

use std::collections::HashSet;

use chrono::Utc;
use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::ops::{operator_role, operator_user, operator_user_role};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use uuid::Uuid;

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

/// Roles assigned to an operator user (via `operator_user_role`).
pub async fn roles_for_user(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> KabiPayResult<Vec<operator_role::Model>> {
    let links = operator_user_role::Entity::find()
        .filter(operator_user_role::Column::OperatorUserId.eq(user_id))
        .all(db)
        .await?;
    let ids: Vec<Uuid> = links.into_iter().map(|l| l.operator_role_id).collect();
    if ids.is_empty() {
        return Ok(vec![]);
    }
    Ok(operator_role::Entity::find()
        .filter(operator_role::Column::Id.is_in(ids))
        .order_by_asc(operator_role::Column::Name)
        .all(db)
        .await?)
}

/// Replace all role assignments for the user with `role_ids` (idempotent, deduped).
pub async fn set_user_roles(
    db: &DatabaseConnection,
    user_id: Uuid,
    role_ids: Vec<Uuid>,
) -> KabiPayResult<()> {
    operator_user::Entity::find_by_id(user_id)
        .one(db)
        .await?
        .filter(|u| !u.is_deleted)
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "operator_user",
            id: user_id.to_string(),
        })?;

    let unique: Vec<Uuid> = role_ids
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();

    for rid in &unique {
        operator_role::Entity::find_by_id(*rid)
            .one(db)
            .await?
            .ok_or_else(|| KabiPayError::NotFound {
                entity: "operator_role",
                id: rid.to_string(),
            })?;
    }

    let txn = db.begin().await?;
    operator_user_role::Entity::delete_many()
        .filter(operator_user_role::Column::OperatorUserId.eq(user_id))
        .exec(&txn)
        .await?;

    let now = Utc::now();
    for rid in unique {
        operator_user_role::ActiveModel {
            operator_user_id: Set(user_id),
            operator_role_id: Set(rid),
            assigned_at: Set(now),
        }
        .insert(&txn)
        .await?;
    }

    txn.commit().await?;
    Ok(())
}
