//! Tenant `master_data` rows for HR-configurable timesheet / attendance settings and catalogs.

use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0028_master_data::master_data;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
    Set,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const CAT_ATTENDANCE_ADJUSTMENT: &str = "HRMS_ATTENDANCE_ADJUSTMENT";
pub const CAT_TIMESHEET_LOCK: &str = "HRMS_TIMESHEET_LOCK";
pub const CAT_TIMESHEET_PROJECT: &str = "TIMESHEET_PROJECT";
pub const CAT_TIMESHEET_TASK: &str = "TIMESHEET_TASK";

pub const KEY_POLICY: &str = "POLICY";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AttendanceAdjustmentPolicy {
    /// Days after `work_date` an employee may still add a manual segment without privileged permission.
    #[serde(default = "default_self_adjust_days")]
    pub max_self_adjust_days: i64,
}

fn default_self_adjust_days() -> i64 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TimesheetLockPolicy {
    /// Number of week periods (Mon–Sun blocks) **including** the current week that stay editable for drafts.
    #[serde(default = "default_editable_week_span")]
    pub editable_week_span: i64,
    #[serde(default = "default_lock_approved")]
    pub lock_approved_entries: bool,
}

fn default_editable_week_span() -> i64 {
    2
}

fn default_lock_approved() -> bool {
    true
}

pub async fn load_attendance_adjustment_policy(
    db: &impl ConnectionTrait,
    tenant_id: Uuid,
) -> KabiPayResult<AttendanceAdjustmentPolicy> {
    let row = master_data::Entity::find()
        .filter(master_data::Column::TenantId.eq(tenant_id))
        .filter(master_data::Column::Category.eq(CAT_ATTENDANCE_ADJUSTMENT))
        .filter(master_data::Column::DataKey.eq(KEY_POLICY))
        .filter(master_data::Column::IsActive.eq(true))
        .one(db)
        .await?;
    let Some(row) = row else {
        return Ok(AttendanceAdjustmentPolicy::default());
    };
    serde_json::from_str(&row.value).map_err(|e| {
        KabiPayError::Validation(format!("invalid attendance adjustment policy JSON: {e}"))
    })
}

pub async fn load_timesheet_lock_policy(
    db: &impl ConnectionTrait,
    tenant_id: Uuid,
) -> KabiPayResult<TimesheetLockPolicy> {
    let row = master_data::Entity::find()
        .filter(master_data::Column::TenantId.eq(tenant_id))
        .filter(master_data::Column::Category.eq(CAT_TIMESHEET_LOCK))
        .filter(master_data::Column::DataKey.eq(KEY_POLICY))
        .filter(master_data::Column::IsActive.eq(true))
        .one(db)
        .await?;
    let Some(row) = row else {
        return Ok(TimesheetLockPolicy::default());
    };
    serde_json::from_str(&row.value).map_err(|e| {
        KabiPayError::Validation(format!("invalid timesheet lock policy JSON: {e}"))
    })
}

pub async fn upsert_policy_json(
    db: &impl ConnectionTrait,
    tenant_id: Uuid,
    category: &str,
    value_json: &str,
) -> KabiPayResult<master_data::Model> {
    let now = chrono::Utc::now();
    let existing = master_data::Entity::find()
        .filter(master_data::Column::TenantId.eq(tenant_id))
        .filter(master_data::Column::Category.eq(category))
        .filter(master_data::Column::DataKey.eq(KEY_POLICY))
        .one(db)
        .await?;

    if let Some(m) = existing {
        let mut am: master_data::ActiveModel = m.into();
        am.value = Set(value_json.to_string());
        am.updated_at = Set(now);
        am.is_active = Set(true);
        let out = am.update(db).await?;
        return Ok(out);
    }

    let id = Uuid::new_v4();
    let am = master_data::ActiveModel {
        id: Set(id),
        tenant_id: Set(tenant_id),
        category: Set(category.into()),
        data_key: Set(KEY_POLICY.into()),
        value: Set(value_json.to_string()),
        description: Set(Some("HRMS policy JSON".into())),
        display_order: Set(Some(0)),
        is_system: Set(false),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    };
    am.insert(db).await.map_err(KabiPayError::from)
}

pub async fn list_projects(
    db: &impl ConnectionTrait,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<master_data::Model>> {
    let limit = limit.clamp(1, 500);
    master_data::Entity::find()
        .filter(master_data::Column::TenantId.eq(tenant_id))
        .filter(master_data::Column::Category.eq(CAT_TIMESHEET_PROJECT))
        .filter(master_data::Column::IsActive.eq(true))
        .order_by_asc(master_data::Column::DisplayOrder)
        .order_by_asc(master_data::Column::DataKey)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

/// `data_key` = project code; `value` = task codes JSON array `["DEV","MEET"]`.
pub async fn list_task_rows_for_project(
    db: &impl ConnectionTrait,
    tenant_id: Uuid,
    project_code: &str,
) -> KabiPayResult<Vec<master_data::Model>> {
    master_data::Entity::find()
        .filter(master_data::Column::TenantId.eq(tenant_id))
        .filter(master_data::Column::Category.eq(CAT_TIMESHEET_TASK))
        .filter(master_data::Column::DataKey.eq(project_code))
        .filter(master_data::Column::IsActive.eq(true))
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn upsert_catalog_row(
    db: &impl ConnectionTrait,
    tenant_id: Uuid,
    category: &str,
    data_key: &str,
    value: &str,
    display_order: Option<i32>,
) -> KabiPayResult<master_data::Model> {
    let now = chrono::Utc::now();
    let existing = master_data::Entity::find()
        .filter(master_data::Column::TenantId.eq(tenant_id))
        .filter(master_data::Column::Category.eq(category))
        .filter(master_data::Column::DataKey.eq(data_key))
        .one(db)
        .await?;

    if let Some(m) = existing {
        let mut am: master_data::ActiveModel = m.into();
        am.value = Set(value.into());
        am.updated_at = Set(now);
        am.is_active = Set(true);
        if let Some(o) = display_order {
            am.display_order = Set(Some(o));
        }
        return Ok(am.update(db).await?);
    }

    let id = Uuid::new_v4();
    let am = master_data::ActiveModel {
        id: Set(id),
        tenant_id: Set(tenant_id),
        category: Set(category.into()),
        data_key: Set(data_key.into()),
        value: Set(value.into()),
        description: Set(None),
        display_order: Set(display_order),
        is_system: Set(false),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
    };
    am.insert(db).await.map_err(KabiPayError::from)
}
