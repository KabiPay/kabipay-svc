//! GraphQL DTOs for kabipay-performance.

use async_graphql::{SimpleObject, ID};
use chrono::{DateTime, NaiveDate, Utc};
use kabipay_db_entities::tenant::d0018_performance::{goal, review_cycle};

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "ReviewCycle")]
pub struct ReviewCycleDto {
    pub id: ID,
    pub tenant_id: ID,
    pub name: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: String,
    pub review_type: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<review_cycle::Model> for ReviewCycleDto {
    fn from(m: review_cycle::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            name: m.name,
            start_date: m.start_date,
            end_date: m.end_date,
            status: m.status,
            review_type: m.review_type,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Goal")]
pub struct GoalDto {
    pub id: ID,
    pub tenant_id: ID,
    pub employee_id: ID,
    pub review_cycle_id: ID,
    pub title: String,
    pub description: Option<String>,
    pub weightage: Option<String>,
    pub status: String,
}

impl From<goal::Model> for GoalDto {
    fn from(m: goal::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            employee_id: ID(m.employee_id.to_string()),
            review_cycle_id: ID(m.review_cycle_id.to_string()),
            title: m.title,
            description: m.description,
            weightage: m.weightage.map(|d| d.to_string()),
            status: m.status,
        }
    }
}
