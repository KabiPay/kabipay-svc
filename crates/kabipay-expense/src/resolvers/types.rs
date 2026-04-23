//! GraphQL DTOs for kabipay-expense.

use async_graphql::{InputObject, SimpleObject, ID};
use chrono::{DateTime, NaiveDate, Utc};
use kabipay_db_entities::tenant::d0015_expense::{expense, expense_category};

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "ExpenseCategory")]
pub struct ExpenseCategoryDto {
    pub id: ID,
    pub tenant_id: ID,
    pub name: String,
    pub code: String,
    pub max_amount_per_claim: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<expense_category::Model> for ExpenseCategoryDto {
    fn from(m: expense_category::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            name: m.name,
            code: m.code,
            max_amount_per_claim: m.max_amount_per_claim.map(|d| d.to_string()),
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Expense")]
pub struct ExpenseDto {
    pub id: ID,
    pub tenant_id: ID,
    pub employee_id: ID,
    pub expense_category_id: ID,
    pub amount: String,
    pub currency: String,
    pub expense_date: NaiveDate,
    pub title: String,
    pub status: String,
    pub submitted_at: DateTime<Utc>,
}

#[derive(InputObject, Clone, Debug)]
pub struct SubmitExpenseInput {
    pub expense_category_id: ID,
    /// String decimal, e.g. "1250.50"
    pub amount: String,
    /// ISO 4217, e.g. "INR"
    pub currency: String,
    pub expense_date: NaiveDate,
    pub title: String,
}

impl From<expense::Model> for ExpenseDto {
    fn from(m: expense::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            employee_id: ID(m.employee_id.to_string()),
            expense_category_id: ID(m.expense_category_id.to_string()),
            amount: m.amount.to_string(),
            currency: m.currency,
            expense_date: m.expense_date,
            title: m.title,
            status: m.status,
            submitted_at: m.submitted_at,
        }
    }
}
