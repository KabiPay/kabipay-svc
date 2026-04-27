//! Tenant-scoped SeaORM queries and commands for expenses.

use chrono::{NaiveDate, Utc};
use kabipay_common::client_data_scope::EmployeeScopeFilter;
use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0007_employee_core::employee;
use kabipay_db_entities::tenant::d0015_expense::{expense, expense_category};
use kabipay_db_entities::tenant::d0033_travel_request::travel_request;
use kabipay_db_entities::tenant::d0027_communication_audit::notification;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use std::str::FromStr;
use uuid::Uuid;

pub async fn list_categories(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<expense_category::Model>> {
    let limit = limit.clamp(1, 200);
    expense_category::Entity::find()
        .filter(expense_category::Column::TenantId.eq(tenant_id))
        .filter(expense_category::Column::IsDeleted.eq(false))
        .order_by_asc(expense_category::Column::Code)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_expenses(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
    scope_filter: &EmployeeScopeFilter,
) -> KabiPayResult<Vec<expense::Model>> {
    let limit = limit.clamp(1, 200);
    match scope_filter {
        EmployeeScopeFilter::Empty => return Ok(vec![]),
        EmployeeScopeFilter::EmployeeIds(ids) if ids.is_empty() => return Ok(vec![]),
        _ => {}
    }
    let mut q = expense::Entity::find()
        .filter(expense::Column::TenantId.eq(tenant_id))
        .filter(expense::Column::IsDeleted.eq(false));
    if let EmployeeScopeFilter::EmployeeIds(ids) = scope_filter {
        q = q.filter(expense::Column::EmployeeId.is_in(ids.clone()));
    }
    q.order_by_desc(expense::Column::SubmittedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

/// Submit a new expense claim in `PENDING` status; validates category belongs to the tenant.
/// Optional `travel_request_id` links the claim to that employee’s trip.
pub async fn submit_expense(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    expense_category_id: Uuid,
    amount: Decimal,
    currency: &str,
    expense_date: NaiveDate,
    title: &str,
    travel_request_id: Option<Uuid>,
) -> KabiPayResult<expense::Model> {
    let _cat = expense_category::Entity::find()
        .filter(expense_category::Column::Id.eq(expense_category_id))
        .filter(expense_category::Column::TenantId.eq(tenant_id))
        .filter(expense_category::Column::IsDeleted.eq(false))
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "expense_category",
            id: expense_category_id.to_string(),
        })?;
    if amount <= Decimal::ZERO {
        return Err(KabiPayError::Validation(
            "amount must be greater than zero".into(),
        ));
    }
    if let Some(tid) = travel_request_id {
        let t = travel_request::Entity::find()
            .filter(travel_request::Column::Id.eq(tid))
            .filter(travel_request::Column::TenantId.eq(tenant_id))
            .one(db)
            .await?
            .ok_or_else(|| KabiPayError::NotFound {
                entity: "travel_request",
                id: tid.to_string(),
            })?;
        if t.employee_id != employee_id {
            return Err(KabiPayError::Validation(
                "travel request must belong to the submitting employee".into(),
            ));
        }
    }
    let id = Uuid::new_v4();
    let now = Utc::now();
    let am = expense::ActiveModel {
        id: Set(id),
        tenant_id: Set(tenant_id),
        employee_id: Set(employee_id),
        expense_category_id: Set(expense_category_id),
        amount: Set(amount),
        currency: Set(currency.to_string()),
        expense_date: Set(expense_date),
        title: Set(title.to_string()),
        status: Set("PENDING".into()),
        travel_request_id: Set(travel_request_id),
        rejection_reason: Set(None),
        approved_by: Set(None),
        workflow_instance_id: Set(None),
        submitted_at: Set(now),
        is_deleted: Set(false),
        deleted_at: Set(None),
        deleted_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    am.insert(db).await?;
    expense::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("inserted expense not found".into()))
}

/// Parse a decimal from string (GraphQL) into `Decimal`.
pub fn parse_amount(s: &str) -> KabiPayResult<Decimal> {
    Decimal::from_str(s.trim())
        .map_err(|_| KabiPayError::Validation("invalid amount; must be a decimal string".into()))
}

const STATUS_PENDING: &str = "PENDING";
const STATUS_APPROVED: &str = "APPROVED";
const STATUS_REJECTED: &str = "REJECTED";

pub async fn approve_expense(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    expense_id: Uuid,
    approver_user_id: Uuid,
) -> KabiPayResult<expense::Model> {
    let model = load_pending_expense(db, tenant_id, expense_id).await?;
    let now = Utc::now();
    let mut am: expense::ActiveModel = model.into();
    am.status = Set(STATUS_APPROVED.into());
    am.rejection_reason = Set(None);
    am.approved_by = Set(Some(approver_user_id));
    am.updated_at = Set(now);
    am.update(db).await?;
    let out = expense::Entity::find_by_id(expense_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("updated expense not found".into()))?;
    expense_notify_employee(
        db,
        tenant_id,
        out.employee_id,
        "Expense approved",
        &format!("Your expense claim \"{}\" was approved.", out.title),
    )
    .await;
    Ok(out)
}

pub async fn reject_expense(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    expense_id: Uuid,
    rejection_reason: Option<String>,
) -> KabiPayResult<expense::Model> {
    let model = load_pending_expense(db, tenant_id, expense_id).await?;
    let now = Utc::now();
    let mut am: expense::ActiveModel = model.into();
    am.status = Set(STATUS_REJECTED.into());
    am.rejection_reason = Set(rejection_reason);
    am.approved_by = Set(None);
    am.updated_at = Set(now);
    am.update(db).await?;
    let out = expense::Entity::find_by_id(expense_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("updated expense not found".into()))?;
    let msg = format!(
        "Your expense claim \"{}\" was rejected.{}",
        out.title,
        match &out.rejection_reason {
            Some(s) if !s.is_empty() => format!(" Reason: {s}"),
            _ => String::new(),
        }
    );
    expense_notify_employee(db, tenant_id, out.employee_id, "Expense rejected", &msg).await;
    Ok(out)
}

async fn load_pending_expense(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    expense_id: Uuid,
) -> KabiPayResult<expense::Model> {
    let m = expense::Entity::find()
        .filter(expense::Column::Id.eq(expense_id))
        .filter(expense::Column::TenantId.eq(tenant_id))
        .filter(expense::Column::IsDeleted.eq(false))
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "expense",
            id: expense_id.to_string(),
        })?;
    if m.status != STATUS_PENDING {
        return Err(KabiPayError::Validation(
            "only PENDING expenses can be approved or rejected".into(),
        ));
    }
    Ok(m)
}

async fn expense_notify_employee(
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
        r#type: Set(Some("EXPENSE".into())),
        title: Set(Some(title.into())),
        message: Set(Some(message.into())),
        action_url: Set(None),
        is_read: Set(false),
        read_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    if let Err(e) = am.insert(db).await {
        tracing::warn!(error = %e, "insert notification (expense) failed");
    }
}
