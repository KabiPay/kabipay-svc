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
    /// When set, this claim is part of a travel trip.
    pub travel_request_id: Option<ID>,
    pub amount: String,
    pub currency: String,
    pub expense_date: NaiveDate,
    pub title: String,
    pub status: String,
    /// Set when **`EXPENSE`** workflow is active with ≥1 step (**M32**).
    pub workflow_instance_id: Option<ID>,
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
    /// Link to a travel request the employee owns (optional).
    pub travel_request_id: Option<ID>,
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "TravelRequest")]
pub struct TravelRequestDto {
    pub id: ID,
    pub tenant_id: ID,
    pub employee_id: ID,
    pub origin_location: Option<String>,
    pub destination_location: Option<String>,
    pub from_date: NaiveDate,
    pub to_date: NaiveDate,
    pub purpose: String,
    pub estimated_amount: Option<String>,
    pub currency: String,
    pub status: String,
    pub rejection_reason: Option<String>,
    pub approved_by: Option<ID>,
    pub rejected_by: Option<ID>,
    pub submitted_at: DateTime<Utc>,
}

#[derive(InputObject, Clone, Debug)]
pub struct SubmitTravelRequestInput {
    pub origin_location: Option<String>,
    pub destination_location: Option<String>,
    pub from_date: NaiveDate,
    pub to_date: NaiveDate,
    pub purpose: String,
    /// Optional string decimal; omit for unknown estimate.
    pub estimated_amount: Option<String>,
    pub currency: String,
}

impl From<kabipay_db_entities::tenant::d0033_travel_request::travel_request::Model> for TravelRequestDto {
    fn from(m: kabipay_db_entities::tenant::d0033_travel_request::travel_request::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            employee_id: ID(m.employee_id.to_string()),
            origin_location: m.origin_location,
            destination_location: m.destination_location,
            from_date: m.from_date,
            to_date: m.to_date,
            purpose: m.purpose,
            estimated_amount: m.estimated_amount.map(|d| d.to_string()),
            currency: m.currency,
            status: m.status,
            rejection_reason: m.rejection_reason,
            approved_by: m.approved_by.map(|u| ID(u.to_string())),
            rejected_by: m.rejected_by.map(|u| ID(u.to_string())),
            submitted_at: m.submitted_at,
        }
    }
}

impl From<expense::Model> for ExpenseDto {
    fn from(m: expense::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            employee_id: ID(m.employee_id.to_string()),
            expense_category_id: ID(m.expense_category_id.to_string()),
            travel_request_id: m.travel_request_id.map(|u| ID(u.to_string())),
            amount: m.amount.to_string(),
            currency: m.currency,
            expense_date: m.expense_date,
            title: m.title,
            status: m.status,
            workflow_instance_id: m.workflow_instance_id.map(|u| ID(u.to_string())),
            submitted_at: m.submitted_at,
        }
    }
}
