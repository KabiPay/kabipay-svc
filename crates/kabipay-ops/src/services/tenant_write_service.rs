//! Mutations for subscriptions, feature flags, module catalog (ops plane).

use chrono::{NaiveDate, Utc};
use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::ops::{feature_flag, module, tenant, tenant_subscription};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use uuid::Uuid;

const SUB_STATUSES: &[&str] = &["PENDING", "ACTIVE", "SUSPENDED", "CANCELLED", "EXPIRED"];
const OVERAGE: &[&str] = &["BLOCK", "ALLOW", "NOTIFY"];

pub async fn upsert_tenant_subscription(
    db: &DatabaseConnection,
    operator_user_id: Uuid,
    tenant_id: Uuid,
    module_id: Uuid,
    status: String,
    contracted_seats: i32,
    overage_policy: String,
    activated_at: Option<NaiveDate>,
    expires_at: Option<NaiveDate>,
) -> KabiPayResult<tenant_subscription::Model> {
    if !SUB_STATUSES.contains(&status.as_str()) {
        return Err(KabiPayError::Validation(format!(
            "invalid subscription status {status}"
        )));
    }
    if !OVERAGE.contains(&overage_policy.as_str()) {
        return Err(KabiPayError::Validation(format!(
            "invalid overage_policy {overage_policy}"
        )));
    }
    if contracted_seats < 0 {
        return Err(KabiPayError::Validation(
            "contracted_seats must be non-negative".into(),
        ));
    }

    tenant::Entity::find_by_id(tenant_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "tenant",
            id: tenant_id.to_string(),
        })?;

    module::Entity::find_by_id(module_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "module",
            id: module_id.to_string(),
        })?;

    let existing = tenant_subscription::Entity::find()
        .filter(tenant_subscription::Column::TenantId.eq(tenant_id))
        .filter(tenant_subscription::Column::ModuleId.eq(module_id))
        .filter(tenant_subscription::Column::IsDeleted.eq(false))
        .one(db)
        .await?;

    let now = Utc::now();
    if let Some(row) = existing {
        let mut am: tenant_subscription::ActiveModel = row.into();
        am.status = Set(status);
        am.contracted_seats = Set(contracted_seats);
        am.overage_policy = Set(overage_policy);
        am.activated_at = Set(activated_at);
        am.expires_at = Set(expires_at);
        am.approved_by = Set(Some(operator_user_id));
        am.updated_at = Set(now);
        let m = am.update(db).await?;
        enforce_seat_cap(&m)?;
        return Ok(m);
    }

    let m = tenant_subscription::ActiveModel {
        id: Set(Uuid::new_v4()),
        tenant_id: Set(tenant_id),
        module_id: Set(module_id),
        status: Set(status),
        activated_at: Set(activated_at),
        expires_at: Set(expires_at),
        contracted_seats: Set(contracted_seats),
        current_seat_usage: Set(0),
        overage_policy: Set(overage_policy),
        approved_by: Set(Some(operator_user_id)),
        is_deleted: Set(false),
        deleted_at: Set(None),
        deleted_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;
    enforce_seat_cap(&m)?;
    Ok(m)
}

fn enforce_seat_cap(m: &tenant_subscription::Model) -> KabiPayResult<()> {
    if m.current_seat_usage > m.contracted_seats && m.overage_policy == "BLOCK" {
        return Err(KabiPayError::SeatLimitReached {
            module_code: m.module_id.to_string(),
            contracted: m.contracted_seats,
            current: m.current_seat_usage,
        });
    }
    Ok(())
}

pub async fn remove_tenant_subscription(
    db: &DatabaseConnection,
    operator_user_id: Uuid,
    subscription_id: Uuid,
) -> KabiPayResult<bool> {
    let row = tenant_subscription::Entity::find_by_id(subscription_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "tenant_subscription",
            id: subscription_id.to_string(),
        })?;
    if row.is_deleted {
        return Ok(false);
    }
    let now = Utc::now();
    let mut am: tenant_subscription::ActiveModel = row.into();
    am.is_deleted = Set(true);
    am.deleted_at = Set(Some(now));
    am.deleted_by = Set(Some(operator_user_id));
    am.status = Set("CANCELLED".into());
    am.updated_at = Set(now);
    am.update(db).await?;
    Ok(true)
}

pub async fn upsert_feature_flag(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    feature_name: String,
    is_enabled: bool,
) -> KabiPayResult<feature_flag::Model> {
    let fname = feature_name.trim();
    if fname.is_empty() || fname.len() > 255 {
        return Err(KabiPayError::Validation(
            "feature_name must be 1–255 characters".into(),
        ));
    }
    let fname = fname.to_string();

    tenant::Entity::find_by_id(tenant_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "tenant",
            id: tenant_id.to_string(),
        })?;

    let existing = feature_flag::Entity::find()
        .filter(feature_flag::Column::TenantId.eq(tenant_id))
        .filter(feature_flag::Column::FeatureName.eq(fname.clone()))
        .one(db)
        .await?;

    let now = Utc::now();
    if let Some(row) = existing {
        let mut am: feature_flag::ActiveModel = row.into();
        am.is_enabled = Set(is_enabled);
        am.updated_at = Set(now);
        return Ok(am.update(db).await?);
    }

    Ok(
        feature_flag::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(tenant_id),
            feature_name: Set(fname),
            is_enabled: Set(is_enabled),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(db)
        .await?,
    )
}

pub async fn set_module_active(
    db: &DatabaseConnection,
    module_id: Uuid,
    is_active: bool,
) -> KabiPayResult<module::Model> {
    let row = module::Entity::find_by_id(module_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "module",
            id: module_id.to_string(),
        })?;
    let mut am: module::ActiveModel = row.into();
    am.is_active = Set(is_active);
    am.updated_at = Set(Utc::now());
    Ok(am.update(db).await?)
}

pub async fn update_tenant_fields(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    name: Option<String>,
    status: Option<String>,
    plan: Option<String>,
) -> KabiPayResult<tenant::Model> {
    const STATUSES: &[&str] = &["PROVISIONING", "ACTIVE", "SUSPENDED", "TERMINATED"];
    if let Some(ref s) = status {
        if !STATUSES.contains(&s.as_str()) {
            return Err(KabiPayError::Validation(format!("invalid tenant status {s}")));
        }
    }

    let row = tenant::Entity::find_by_id(tenant_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "tenant",
            id: tenant_id.to_string(),
        })?;

    let mut am: tenant::ActiveModel = row.into();
    if let Some(n) = name {
        if n.trim().is_empty() {
            return Err(KabiPayError::Validation("name must not be empty".into()));
        }
        am.name = Set(n);
    }
    if let Some(s) = status {
        am.status = Set(s);
    }
    if let Some(p) = plan {
        am.plan = Set(Some(p));
    }
    am.updated_at = Set(Utc::now());
    Ok(am.update(db).await?)
}

pub async fn list_feature_flags(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<feature_flag::Model>> {
    let limit = limit.clamp(1, 500);
    Ok(feature_flag::Entity::find()
        .filter(feature_flag::Column::TenantId.eq(tenant_id))
        .order_by_asc(feature_flag::Column::FeatureName)
        .limit(limit)
        .all(db)
        .await?)
}
