//! Resolve tenant RBAC for JWT claims: `user_role` + `role` + `role_permission` + `permission`.

use std::collections::{BTreeSet, HashMap};

use kabipay_common::context::ScopeType;
use kabipay_common::KabiPayResult;
use kabipay_db_entities::tenant::d0005_auth_rbac::{
    permission, permission_scope, role, role_permission, user_role,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use uuid::Uuid;

/// Returns `(role_names, permission_codes)` for `user_id` in the tenant `DatabaseConnection`
/// (already schema-scoped). Permission codes are `resource:action` for JWT claims.
pub async fn load_client_rbac(
    db: &DatabaseConnection,
    user_id: Uuid,
) -> KabiPayResult<(Vec<String>, Vec<String>)> {
    let urs = user_role::Entity::find()
        .filter(user_role::Column::UserId.eq(user_id))
        .all(db)
        .await?;
    if urs.is_empty() {
        return Ok((vec![], vec![]));
    }
    let role_ids: Vec<Uuid> = urs.iter().map(|r| r.role_id).collect();

    let role_rows = role::Entity::find()
        .filter(role::Column::Id.is_in(role_ids.clone()))
        .filter(role::Column::IsDeleted.eq(false))
        .all(db)
        .await?;
    let role_names: Vec<String> = role_rows.into_iter().map(|r| r.name).collect();

    let rps = role_permission::Entity::find()
        .filter(role_permission::Column::RoleId.is_in(role_ids))
        .all(db)
        .await?;
    let perm_ids: Vec<Uuid> = rps.iter().map(|p| p.permission_id).collect();
    if perm_ids.is_empty() {
        return Ok((role_names, vec![]));
    }
    let perm_rows = permission::Entity::find()
        .filter(permission::Column::Id.is_in(perm_ids))
        .all(db)
        .await?;
    let set: BTreeSet<String> = perm_rows
        .into_iter()
        .map(|p| format!("{}:{}", p.resource, p.action))
        .collect();
    Ok((role_names, set.into_iter().collect()))
}

/// Merge `permission_scope` rows for the user's roles into a map: `resource` → widest
/// `ScopeType` (wire string for the JWT `resource_scopes` claim).
pub async fn load_client_resource_scopes(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    user_id: Uuid,
) -> KabiPayResult<HashMap<String, String>> {
    let urs = user_role::Entity::find()
        .filter(user_role::Column::UserId.eq(user_id))
        .all(db)
        .await?;
    if urs.is_empty() {
        return Ok(HashMap::new());
    }
    let role_ids: Vec<Uuid> = urs.iter().map(|r| r.role_id).collect();

    let rows = permission_scope::Entity::find()
        .filter(permission_scope::Column::TenantId.eq(tenant_id))
        .filter(permission_scope::Column::RoleId.is_in(role_ids))
        .all(db)
        .await?;

    let mut best: HashMap<String, ScopeType> = HashMap::new();
    for r in rows {
        let Some(s) = ScopeType::parse_loose(&r.scope_type) else {
            continue;
        };
        best.entry(r.resource)
            .and_modify(|e| {
                if s.rank() > e.rank() {
                    *e = s;
                }
            })
            .or_insert(s);
    }
    Ok(best
        .into_iter()
        .map(|(k, v)| (k, v.to_wire().to_string()))
        .collect())
}
