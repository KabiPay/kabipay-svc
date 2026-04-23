//! Tenant-scoped SeaORM queries and commands for expenses.

use chrono::{NaiveDate, Utc};
use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0015_expense::{expense, expense_category};
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
) -> KabiPayResult<Vec<expense::Model>> {
    let limit = limit.clamp(1, 200);
    expense::Entity::find()
        .filter(expense::Column::TenantId.eq(tenant_id))
        .filter(expense::Column::IsDeleted.eq(false))
        .order_by_desc(expense::Column::SubmittedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

/// Submit a new expense claim in `PENDING` status; validates category belongs to the tenant.
pub async fn submit_expense(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    expense_category_id: Uuid,
    amount: Decimal,
    currency: &str,
    expense_date: NaiveDate,
    title: &str,
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
