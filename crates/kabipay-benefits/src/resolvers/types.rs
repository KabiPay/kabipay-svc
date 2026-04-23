//! GraphQL DTOs for kabipay-benefits.

use async_graphql::{SimpleObject, ID};
use chrono::{DateTime, Utc};
use kabipay_db_entities::tenant::d0014_benefits::{benefit_plan, benefit_type};

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "BenefitType")]
pub struct BenefitTypeDto {
    pub id: ID,
    pub tenant_id: ID,
    pub name: String,
    pub code: String,
    pub category: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<benefit_type::Model> for BenefitTypeDto {
    fn from(m: benefit_type::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            name: m.name,
            code: m.code,
            category: m.category,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "BenefitPlan")]
pub struct BenefitPlanDto {
    pub id: ID,
    pub tenant_id: ID,
    pub benefit_type_id: ID,
    pub name: String,
    pub employer_contribution: Option<String>,
    pub employee_contribution: Option<String>,
    pub contribution_type: Option<String>,
    pub is_mandatory: bool,
    pub is_active: bool,
}

impl From<benefit_plan::Model> for BenefitPlanDto {
    fn from(m: benefit_plan::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            benefit_type_id: ID(m.benefit_type_id.to_string()),
            name: m.name,
            employer_contribution: m.employer_contribution.map(|d| d.to_string()),
            employee_contribution: m.employee_contribution.map(|d| d.to_string()),
            contribution_type: m.contribution_type,
            is_mandatory: m.is_mandatory,
            is_active: m.is_active,
        }
    }
}
