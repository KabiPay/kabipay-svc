//! Read-only access to org document metadata (`document_type`, `employee_document`).

use kabipay_common::{KabiPayError, KabiPayResult};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use uuid::Uuid;

use crate::entities::d0008_document_system::{document_type, employee_document};

pub async fn list_document_types(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<document_type::Model>> {
    let limit = limit.clamp(1, 200);
    document_type::Entity::find()
        .filter(document_type::Column::TenantId.eq(tenant_id))
        .filter(document_type::Column::IsDeleted.eq(false))
        .order_by_asc(document_type::Column::Name)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_employee_documents(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<employee_document::Model>> {
    let limit = limit.clamp(1, 200);
    employee_document::Entity::find()
        .filter(employee_document::Column::TenantId.eq(tenant_id))
        .filter(employee_document::Column::EmployeeId.eq(employee_id))
        .filter(employee_document::Column::IsDeleted.eq(false))
        .order_by_desc(employee_document::Column::UpdatedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}
