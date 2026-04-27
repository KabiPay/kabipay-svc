//! Tenant-scoped travel requests (M14).

use chrono::Utc;
use kabipay_common::client_data_scope::EmployeeScopeFilter;
use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0007_employee_core::employee;
use kabipay_db_entities::tenant::d0027_communication_audit::notification;
use kabipay_db_entities::tenant::d0033_travel_request::travel_request;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use uuid::Uuid;

const STATUS_PENDING: &str = "PENDING";
const STATUS_APPROVED: &str = "APPROVED";
const STATUS_REJECTED: &str = "REJECTED";

pub async fn list_travel_requests(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
    scope_filter: &EmployeeScopeFilter,
) -> KabiPayResult<Vec<travel_request::Model>> {
    let limit = limit.clamp(1, 200);
    match scope_filter {
        EmployeeScopeFilter::Empty => return Ok(vec![]),
        EmployeeScopeFilter::EmployeeIds(ids) if ids.is_empty() => return Ok(vec![]),
        _ => {}
    }
    let mut q = travel_request::Entity::find().filter(travel_request::Column::TenantId.eq(tenant_id));
    if let EmployeeScopeFilter::EmployeeIds(ids) = scope_filter {
        q = q.filter(travel_request::Column::EmployeeId.is_in(ids.clone()));
    }
    q.order_by_desc(travel_request::Column::SubmittedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn submit_travel_request(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    origin_location: Option<String>,
    destination_location: Option<String>,
    from_date: chrono::NaiveDate,
    to_date: chrono::NaiveDate,
    purpose: &str,
    estimated_amount: Option<Decimal>,
    currency: &str,
) -> KabiPayResult<travel_request::Model> {
    if from_date > to_date {
        return Err(KabiPayError::Validation(
            "from_date must be on or before to_date".into(),
        ));
    }
    if purpose.trim().is_empty() {
        return Err(KabiPayError::Validation("purpose is required".into()));
    }
    if let Some(a) = estimated_amount {
        if a < Decimal::ZERO {
            return Err(KabiPayError::Validation(
                "estimated_amount cannot be negative".into(),
            ));
        }
    }
    let id = Uuid::new_v4();
    let now = Utc::now();
    let am = travel_request::ActiveModel {
        id: Set(id),
        tenant_id: Set(tenant_id),
        employee_id: Set(employee_id),
        origin_location: Set(origin_location.filter(|s| !s.is_empty())),
        destination_location: Set(destination_location.filter(|s| !s.is_empty())),
        from_date: Set(from_date),
        to_date: Set(to_date),
        purpose: Set(purpose.trim().to_string()),
        estimated_amount: Set(estimated_amount),
        currency: Set(currency.to_string()),
        status: Set(STATUS_PENDING.into()),
        rejection_reason: Set(None),
        approved_by: Set(None),
        rejected_by: Set(None),
        submitted_at: Set(now),
        created_at: Set(now),
        updated_at: Set(now),
    };
    am.insert(db).await?;
    travel_request::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("inserted travel_request not found".into()))
}

pub async fn approve_travel_request(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    travel_request_id: Uuid,
    approver_user_id: Uuid,
) -> KabiPayResult<travel_request::Model> {
    let model = load_pending_travel(db, tenant_id, travel_request_id).await?;
    let now = Utc::now();
    let mut am: travel_request::ActiveModel = model.into();
    am.status = Set(STATUS_APPROVED.into());
    am.rejection_reason = Set(None);
    am.approved_by = Set(Some(approver_user_id));
    am.rejected_by = Set(None);
    am.updated_at = Set(now);
    am.update(db).await?;
    let out = travel_request::Entity::find_by_id(travel_request_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("updated travel_request not found".into()))?;
    travel_notify_employee(
        db,
        tenant_id,
        out.employee_id,
        "Travel request approved",
        "Your travel request was approved.",
    )
    .await;
    Ok(out)
}

pub async fn reject_travel_request(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    travel_request_id: Uuid,
    rejector_user_id: Uuid,
    rejection_reason: Option<String>,
) -> KabiPayResult<travel_request::Model> {
    let model = load_pending_travel(db, tenant_id, travel_request_id).await?;
    let now = Utc::now();
    let mut am: travel_request::ActiveModel = model.into();
    am.status = Set(STATUS_REJECTED.into());
    am.rejection_reason = Set(rejection_reason.clone());
    am.approved_by = Set(None);
    am.rejected_by = Set(Some(rejector_user_id));
    am.updated_at = Set(now);
    am.update(db).await?;
    let out = travel_request::Entity::find_by_id(travel_request_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("updated travel_request not found".into()))?;
    let msg = format!(
        "Your travel request was rejected.{}",
        match &out.rejection_reason {
            Some(s) if !s.is_empty() => format!(" Reason: {s}"),
            _ => String::new(),
        }
    );
    travel_notify_employee(
        db,
        tenant_id,
        out.employee_id,
        "Travel request rejected",
        &msg,
    )
    .await;
    Ok(out)
}

async fn load_pending_travel(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    id: Uuid,
) -> KabiPayResult<travel_request::Model> {
    let m = travel_request::Entity::find()
        .filter(travel_request::Column::Id.eq(id))
        .filter(travel_request::Column::TenantId.eq(tenant_id))
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "travel_request",
            id: id.to_string(),
        })?;
    if m.status != STATUS_PENDING {
        return Err(KabiPayError::Validation(
            "only PENDING travel requests can be approved or rejected".into(),
        ));
    }
    Ok(m)
}

async fn travel_notify_employee(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    title: &str,
    message: &str,
) {
    let user_id: Option<Uuid> = match employee::Entity::find_by_id(employee_id)
        .filter(employee::Column::TenantId.eq(tenant_id))
        .filter(employee::Column::IsDeleted.eq(false))
        .one(db)
        .await
    {
        Ok(Some(emp)) => emp.user_id,
        _ => None,
    };
    let Some(user_id) = user_id else {
        return;
    };
    let now = Utc::now();
    let am = notification::ActiveModel {
        id: Set(Uuid::new_v4()),
        tenant_id: Set(tenant_id),
        user_id: Set(user_id),
        r#type: Set(Some("TRAVEL".into())),
        title: Set(Some(title.into())),
        message: Set(Some(message.into())),
        action_url: Set(None),
        is_read: Set(false),
        read_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    if let Err(e) = am.insert(db).await {
        tracing::warn!(error = %e, "insert notification (travel) failed");
    }
}
