//! Ops-plane SeaORM queries for billing.

use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::ops::{invoice, payment};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};
use uuid::Uuid;

pub async fn list_invoices(
    db: &DatabaseConnection,
    tenant_id: Option<Uuid>,
    limit: u64,
) -> KabiPayResult<Vec<invoice::Model>> {
    let limit = limit.clamp(1, 500);
    let mut q = invoice::Entity::find().filter(invoice::Column::IsDeleted.eq(false));
    if let Some(t) = tenant_id {
        q = q.filter(invoice::Column::TenantId.eq(t));
    }
    q.order_by_desc(invoice::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_payments(
    db: &DatabaseConnection,
    invoice_id: Option<Uuid>,
    limit: u64,
) -> KabiPayResult<Vec<payment::Model>> {
    let limit = limit.clamp(1, 500);
    let mut q = payment::Entity::find();
    if let Some(inv) = invoice_id {
        q = q.filter(payment::Column::InvoiceId.eq(inv));
    }
    q.order_by_desc(payment::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}
