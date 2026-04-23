//! GraphQL DTOs for kabipay-grievance.

use async_graphql::{SimpleObject, ID};
use chrono::{DateTime, Utc};
use kabipay_db_entities::tenant::d0023_grievance::{grievance_case, grievance_category};

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "GrievanceCategory")]
pub struct GrievanceCategoryDto {
    pub id: ID,
    pub tenant_id: ID,
    pub name: String,
    pub code: String,
    pub is_posh: bool,
    pub resolution_sla_days: Option<i32>,
}

impl From<grievance_category::Model> for GrievanceCategoryDto {
    fn from(m: grievance_category::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            name: m.name,
            code: m.code,
            is_posh: m.is_posh,
            resolution_sla_days: m.resolution_sla_days,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "GrievanceCase")]
pub struct GrievanceCaseDto {
    pub id: ID,
    pub tenant_id: ID,
    pub employee_id: ID,
    pub grievance_category_id: ID,
    pub subject: String,
    pub status: String,
    pub priority: Option<String>,
    pub confidentiality_level: Option<String>,
    pub filed_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

impl From<grievance_case::Model> for GrievanceCaseDto {
    fn from(m: grievance_case::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            employee_id: ID(m.employee_id.to_string()),
            grievance_category_id: ID(m.grievance_category_id.to_string()),
            subject: m.subject,
            status: m.status,
            priority: m.priority,
            confidentiality_level: m.confidentiality_level,
            filed_at: m.filed_at,
            resolved_at: m.resolved_at,
        }
    }
}
