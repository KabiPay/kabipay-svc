//! Onboarding checklist rows (domain 0017).

use chrono::Utc;
use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0017_onboarding_offboarding::onboarding_checklist;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use uuid::Uuid;

pub async fn list_checklist_for_employee(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<onboarding_checklist::Model>> {
    let limit = limit.clamp(1, 500);
    onboarding_checklist::Entity::find()
        .filter(onboarding_checklist::Column::TenantId.eq(tenant_id))
        .filter(onboarding_checklist::Column::EmployeeId.eq(employee_id))
        .order_by_asc(onboarding_checklist::Column::DueDate)
        .order_by_asc(onboarding_checklist::Column::TaskName)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn set_task_completed(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    task_id: Uuid,
    is_completed: bool,
) -> KabiPayResult<onboarding_checklist::Model> {
    let row = onboarding_checklist::Entity::find()
        .filter(onboarding_checklist::Column::Id.eq(task_id))
        .filter(onboarding_checklist::Column::TenantId.eq(tenant_id))
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "onboarding_checklist",
            id: task_id.to_string(),
        })?;
    let now = Utc::now();
    let mut am: onboarding_checklist::ActiveModel = row.into();
    am.is_completed = Set(is_completed);
    am.completed_at = Set(if is_completed { Some(now) } else { None });
    am.updated_at = Set(now);
    am.update(db).await?;
    onboarding_checklist::Entity::find_by_id(task_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("onboarding checklist row missing after update".into()))
}
