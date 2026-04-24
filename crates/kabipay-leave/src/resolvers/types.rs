//! GraphQL DTOs for kabipay-leave.

use async_graphql::{InputObject, SimpleObject, ID};
use chrono::{DateTime, NaiveDate, Utc};
use kabipay_db_entities::tenant::d0011_leave::{leave_balance, leave_request, leave_type};

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "LeaveType")]
pub struct LeaveTypeDto {
    pub id: ID,
    pub tenant_id: ID,
    pub name: String,
    pub code: String,
    pub is_paid: bool,
    pub carry_forward: bool,
    pub max_carry_forward_days: Option<i32>,
    pub half_day_allowed: bool,
    pub requires_document: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<leave_type::Model> for LeaveTypeDto {
    fn from(m: leave_type::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            name: m.name,
            code: m.code,
            is_paid: m.is_paid,
            carry_forward: m.carry_forward,
            max_carry_forward_days: m.max_carry_forward_days,
            half_day_allowed: m.half_day_allowed,
            requires_document: m.requires_document,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "LeaveRequest")]
pub struct LeaveRequestDto {
    pub id: ID,
    pub tenant_id: ID,
    pub employee_id: ID,
    pub leave_type_id: ID,
    pub from_date: NaiveDate,
    pub to_date: NaiveDate,
    /// Days requested, serialised as a decimal string for lossless transport.
    pub days_requested: String,
    pub is_half_day: bool,
    pub status: String,
    pub reason: Option<String>,
    pub applied_at: DateTime<Utc>,
    /// Set when tenant has an active **LEAVE_REQUEST** workflow with at least one step (M8).
    pub workflow_instance_id: Option<ID>,
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "LeaveBalance")]
pub struct LeaveBalanceDto {
    pub id: ID,
    pub tenant_id: ID,
    pub employee_id: ID,
    pub leave_type_id: ID,
    pub year: i32,
    pub entitled_days: String,
    pub used_days: String,
    pub pending_days: String,
    pub carried_forward_days: String,
    pub balance_days: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<leave_balance::Model> for LeaveBalanceDto {
    fn from(m: leave_balance::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            employee_id: ID(m.employee_id.to_string()),
            leave_type_id: ID(m.leave_type_id.to_string()),
            year: m.year,
            entitled_days: m.entitled_days.to_string(),
            used_days: m.used_days.to_string(),
            pending_days: m.pending_days.to_string(),
            carried_forward_days: m.carried_forward_days.to_string(),
            balance_days: m.balance_days.to_string(),
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(InputObject, Clone, Debug)]
pub struct SubmitLeaveRequestInput {
    pub leave_type_id: ID,
    pub from_date: NaiveDate,
    pub to_date: NaiveDate,
    pub is_half_day: bool,
    pub half_day_session: Option<String>,
    pub reason: Option<String>,
}

impl From<leave_request::Model> for LeaveRequestDto {
    fn from(m: leave_request::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            employee_id: ID(m.employee_id.to_string()),
            leave_type_id: ID(m.leave_type_id.to_string()),
            from_date: m.from_date,
            to_date: m.to_date,
            days_requested: m.days_requested.to_string(),
            is_half_day: m.is_half_day,
            status: m.status,
            reason: m.reason,
            applied_at: m.applied_at,
            workflow_instance_id: m.workflow_instance_id.map(|u| ID(u.to_string())),
        }
    }
}
