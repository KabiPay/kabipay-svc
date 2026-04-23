//! GraphQL DTOs for kabipay-operator (ops plane).

use async_graphql::{SimpleObject, ID};
use chrono::{DateTime, Utc};
use kabipay_db_entities::ops::{operator_role, operator_user};

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "OperatorUser")]
pub struct OperatorUserDto {
    pub id: ID,
    pub email: String,
    pub full_name: String,
    pub phone: Option<String>,
    pub is_active: bool,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<operator_user::Model> for OperatorUserDto {
    fn from(m: operator_user::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            email: m.email,
            full_name: m.full_name,
            phone: m.phone,
            is_active: m.is_active,
            last_login_at: m.last_login_at,
            created_at: m.created_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "OperatorRole")]
pub struct OperatorRoleDto {
    pub id: ID,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<operator_role::Model> for OperatorRoleDto {
    fn from(m: operator_role::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            code: m.code,
            name: m.name,
            description: m.description,
            created_at: m.created_at,
        }
    }
}
