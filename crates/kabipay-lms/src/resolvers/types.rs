//! GraphQL DTOs for kabipay-lms.

use async_graphql::{SimpleObject, ID};
use chrono::{DateTime, Utc};
use kabipay_db_entities::tenant::d0019_lms::{course, skill};

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Skill")]
pub struct SkillDto {
    pub id: ID,
    pub tenant_id: ID,
    pub name: String,
    pub category: Option<String>,
    pub level: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<skill::Model> for SkillDto {
    fn from(m: skill::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            name: m.name,
            category: m.category,
            level: m.level,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Course")]
pub struct CourseDto {
    pub id: ID,
    pub tenant_id: ID,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub delivery_mode: Option<String>,
    pub duration_minutes: Option<i32>,
    pub is_mandatory: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<course::Model> for CourseDto {
    fn from(m: course::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            title: m.title,
            description: m.description,
            category: m.category,
            delivery_mode: m.delivery_mode,
            duration_minutes: m.duration_minutes,
            is_mandatory: m.is_mandatory,
            is_active: m.is_active,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}
