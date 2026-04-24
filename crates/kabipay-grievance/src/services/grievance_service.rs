//! Tenant-scoped SeaORM queries for grievance cases.

use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0023_grievance::{grievance_case, grievance_category};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
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
    employee_id_filter: Option<Uuid>,
) -> KabiPayResult<Vec<grievance_case::Model>> {
    let limit = limit.clamp(1, 500);
    let mut q = grievance_case::Entity::find()
        .filter(grievance_case::Column::TenantId.eq(tenant_id));
    if let Some(eid) = employee_id_filter {
        q = q.filter(grievance_case::Column::EmployeeId.eq(eid));
    }
    q.order_by_desc(grievance_case::Column::FiledAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn submit_case(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: uuid::Uuid,
    grievance_category_id: uuid::Uuid,
    subject: &str,
    description: Option<&str>,
) -> KabiPayResult<grievance_case::Model> {
    if subject.trim().is_empty() {
        return Err(KabiPayError::Validation("subject is required".into()));
    }
    let _cat = grievance_category::Entity::find()
        .filter(grievance_category::Column::Id.eq(grievance_category_id))
        .filter(grievance_category::Column::TenantId.eq(tenant_id))
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "grievance_category",
            id: grievance_category_id.to_string(),
        })?;
    let id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let desc = description.and_then(|s| {
        let t = s.trim();
        if t.is_empty() {
            None
        } else {
            Some(t.to_string())
        }
    });
    let am = grievance_case::ActiveModel {
        id: Set(id),
        tenant_id: Set(tenant_id),
        employee_id: Set(employee_id),
        grievance_category_id: Set(grievance_category_id),
        subject: Set(subject.trim().to_string()),
        description: Set(desc),
        status: Set("OPEN".into()),
        priority: Set(None),
        confidentiality_level: Set(None),
        assigned_to: Set(None),
        filed_at: Set(now),
        resolved_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    am.insert(db).await?;
    grievance_case::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("inserted grievance_case not found".into()))
}
