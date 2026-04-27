//! Tenant-scoped reads for analytics domain (0024) and outbox listing (0030, HR only at resolver).

use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0024_analytics::{
    dashboard, dashboard_widget, report_definition, report_schedule, workforce_snapshot,
};
use kabipay_db_entities::tenant::d0030_outbox_events::outbox_event;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use uuid::Uuid;

pub async fn list_report_definitions(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<report_definition::Model>> {
    let limit = limit.clamp(1, 200);
    report_definition::Entity::find()
        .filter(report_definition::Column::TenantId.eq(tenant_id))
        .order_by_asc(report_definition::Column::Name)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_report_schedules(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<report_schedule::Model>> {
    let limit = limit.clamp(1, 200);
    report_schedule::Entity::find()
        .filter(report_schedule::Column::TenantId.eq(tenant_id))
        .order_by_desc(report_schedule::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_dashboards(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<dashboard::Model>> {
    let limit = limit.clamp(1, 100);
    dashboard::Entity::find()
        .filter(dashboard::Column::TenantId.eq(tenant_id))
        .order_by_asc(dashboard::Column::Name)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_dashboard_widgets(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    dashboard_id: Option<Uuid>,
    limit: u64,
) -> KabiPayResult<Vec<dashboard_widget::Model>> {
    let limit = limit.clamp(1, 200);
    let mut q = dashboard_widget::Entity::find()
        .filter(dashboard_widget::Column::TenantId.eq(tenant_id));
    if let Some(did) = dashboard_id {
        q = q.filter(dashboard_widget::Column::DashboardId.eq(did));
    }
    q.order_by_asc(dashboard_widget::Column::GridRow)
        .order_by_asc(dashboard_widget::Column::GridCol)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_workforce_snapshots(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<workforce_snapshot::Model>> {
    let limit = limit.clamp(1, 120);
    workforce_snapshot::Entity::find()
        .filter(workforce_snapshot::Column::TenantId.eq(tenant_id))
        .order_by_desc(workforce_snapshot::Column::SnapshotDate)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_outbox_events(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    status: Option<String>,
    limit: u64,
) -> KabiPayResult<Vec<outbox_event::Model>> {
    let limit = limit.clamp(1, 500);
    let mut q = outbox_event::Entity::find().filter(outbox_event::Column::TenantId.eq(tenant_id));
    if let Some(s) = status {
        let t = s.trim();
        if !t.is_empty() {
            q = q.filter(outbox_event::Column::Status.eq(t));
        }
    }
    q.order_by_desc(outbox_event::Column::CreatedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

const OB_PENDING: &str = "PENDING";
const OB_FAILED: &str = "FAILED";
const OB_PROCESSING: &str = "PROCESSING";

/// HR: send a **FAILED** or stuck **PROCESSING** row back to **PENDING** for the worker to pick up.
pub async fn requeue_outbox_event(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    id: Uuid,
) -> KabiPayResult<outbox_event::Model> {
    let m = outbox_event::Entity::find_by_id(id)
        .filter(outbox_event::Column::TenantId.eq(tenant_id))
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "outbox_event",
            id: id.to_string(),
        })?;
    if m.status != OB_FAILED && m.status != OB_PROCESSING {
        return Err(KabiPayError::Validation(
            "only FAILED or PROCESSING outbox events can be requeued".into(),
        ));
    }
    let note = " [manual requeue]";
    let prev_err = m.last_error.clone();
    let err = prev_err
        .map(|e| {
            let s = format!("{e}{note}");
            if s.len() > 2000 {
                format!("{}…", &s[..1997])
            } else {
                s
            }
        })
        .unwrap_or_else(|| "manual requeue".to_string());
    let mut am: outbox_event::ActiveModel = m.into();
    am.status = Set(OB_PENDING.into());
    am.processed_at = Set(None);
    am.claimed_at = Set(None);
    am.last_error = Set(Some(err));
    am.update(db).await?;
    outbox_event::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("outbox row missing after requeue".into()))
}
