//! Tenant-scoped SeaORM queries for workflow definitions and runtime.

use chrono::Utc;
use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0025_workflow::{workflow, workflow_instance, workflow_step};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use uuid::Uuid;

pub async fn list_workflows(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<workflow::Model>> {
    let limit = limit.clamp(1, 200);
    workflow::Entity::find()
        .filter(workflow::Column::TenantId.eq(tenant_id))
        .filter(workflow::Column::IsActive.eq(true))
        .order_by_asc(workflow::Column::Name)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_instances(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<workflow_instance::Model>> {
    let limit = limit.clamp(1, 500);
    workflow_instance::Entity::find()
        .filter(workflow_instance::Column::TenantId.eq(tenant_id))
        .order_by_desc(workflow_instance::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

/// Ordered steps for a workflow (definition).
pub async fn list_workflow_steps(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    workflow_id: Uuid,
) -> KabiPayResult<Vec<workflow_step::Model>> {
    workflow_step::Entity::find()
        .filter(workflow_step::Column::TenantId.eq(tenant_id))
        .filter(workflow_step::Column::WorkflowId.eq(workflow_id))
        .order_by_asc(workflow_step::Column::SequenceOrder)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn get_workflow(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    workflow_id: Uuid,
) -> KabiPayResult<Option<workflow::Model>> {
    workflow::Entity::find()
        .filter(workflow::Column::TenantId.eq(tenant_id))
        .filter(workflow::Column::Id.eq(workflow_id))
        .one(db)
        .await
        .map_err(KabiPayError::from)
}

/// HR / tenant admin: new approval **definition** row.
pub async fn create_workflow(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    name: String,
    entity_type: String,
    is_active: bool,
) -> KabiPayResult<workflow::Model> {
    let id = Uuid::new_v4();
    let now = Utc::now();
    let m = workflow::ActiveModel {
        id: Set(id),
        tenant_id: Set(tenant_id),
        name: Set(name),
        entity_type: Set(entity_type),
        is_active: Set(is_active),
        created_at: Set(now),
        updated_at: Set(now),
    };
    m.insert(db).await.map_err(KabiPayError::from)
}

/// Append a **step** to a definition. Fails if `workflow_id` is not in tenant.
pub async fn create_workflow_step(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    workflow_id: Uuid,
    sequence_order: i32,
    step_name: String,
    approver_type: Option<String>,
    approver_role_id: Option<Uuid>,
    can_skip: bool,
    sla_hours: Option<i32>,
) -> KabiPayResult<workflow_step::Model> {
    if get_workflow(db, tenant_id, workflow_id).await?.is_none() {
        return Err(KabiPayError::NotFound {
            entity: "workflow",
            id: workflow_id.to_string(),
        });
    }

    let dupe = workflow_step::Entity::find()
        .filter(workflow_step::Column::TenantId.eq(tenant_id))
        .filter(workflow_step::Column::WorkflowId.eq(workflow_id))
        .filter(workflow_step::Column::SequenceOrder.eq(sequence_order))
        .one(db)
        .await
        .map_err(KabiPayError::from)?;
    if dupe.is_some() {
        return Err(KabiPayError::Validation(format!(
            "workflow step with sequence_order {sequence_order} already exists for this workflow"
        )));
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    let m = workflow_step::ActiveModel {
        id: Set(id),
        tenant_id: Set(tenant_id),
        workflow_id: Set(workflow_id),
        sequence_order: Set(sequence_order),
        step_name: Set(step_name),
        approver_type: Set(approver_type),
        approver_role_id: Set(approver_role_id),
        can_skip: Set(can_skip),
        sla_hours: Set(sla_hours),
        created_at: Set(now),
        updated_at: Set(now),
    };
    m.insert(db).await.map_err(KabiPayError::from)
}
