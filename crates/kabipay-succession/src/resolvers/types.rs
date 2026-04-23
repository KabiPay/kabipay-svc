//! GraphQL DTOs for kabipay-succession.

use async_graphql::{SimpleObject, ID};
use chrono::{DateTime, Utc};
use kabipay_db_entities::tenant::d0020_succession::{competency, talent_pool};

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Competency")]
pub struct CompetencyDto {
    pub id: ID,
    pub tenant_id: ID,
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<competency::Model> for CompetencyDto {
    fn from(m: competency::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            name: m.name,
            category: m.category,
            description: m.description,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "TalentPool")]
pub struct TalentPoolDto {
    pub id: ID,
    pub tenant_id: ID,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<talent_pool::Model> for TalentPoolDto {
    fn from(m: talent_pool::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            name: m.name,
            description: m.description,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}
