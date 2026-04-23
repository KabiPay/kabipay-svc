//! GraphQL DTOs for kabipay-tenant (ops plane).

use async_graphql::{SimpleObject, ID};
use chrono::{DateTime, NaiveDate, Utc};
use kabipay_db_entities::ops::{module, tenant, tenant_subscription};

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Tenant")]
pub struct TenantDto {
    pub id: ID,
    pub name: String,
    pub status: String,
    pub plan: Option<String>,
    pub country: Option<String>,
    pub currency: Option<String>,
    pub subdomain: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<tenant::Model> for TenantDto {
    fn from(m: tenant::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            name: m.name,
            status: m.status,
            plan: m.plan,
            country: m.country,
            currency: m.currency,
            subdomain: m.subdomain,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Module")]
pub struct ModuleDto {
    pub id: ID,
    pub code: String,
    pub name: String,
    pub category: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
    pub display_order: i32,
    pub is_core: bool,
}

impl From<module::Model> for ModuleDto {
    fn from(m: module::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            code: m.code,
            name: m.name,
            category: m.category,
            description: m.description,
            is_active: m.is_active,
            display_order: m.display_order,
            is_core: m.is_core,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "TenantSubscription")]
pub struct TenantSubscriptionDto {
    pub id: ID,
    pub tenant_id: ID,
    pub module_id: ID,
    pub status: String,
    pub activated_at: Option<NaiveDate>,
    pub expires_at: Option<NaiveDate>,
    pub contracted_seats: i32,
    pub current_seat_usage: i32,
    pub overage_policy: String,
}

impl From<tenant_subscription::Model> for TenantSubscriptionDto {
    fn from(m: tenant_subscription::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            module_id: ID(m.module_id.to_string()),
            status: m.status,
            activated_at: m.activated_at,
            expires_at: m.expires_at,
            contracted_seats: m.contracted_seats,
            current_seat_usage: m.current_seat_usage,
            overage_policy: m.overage_policy,
        }
    }
}
