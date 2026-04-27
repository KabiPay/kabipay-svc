//! Tenant-scoped SeaORM queries for workflow definitions and runtime.

use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0025_workflow::{workflow, workflow_instance, workflow_step};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
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
