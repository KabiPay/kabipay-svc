//! GraphQL DTOs for kabipay-compensation.

use async_graphql::{SimpleObject, ID};
use chrono::NaiveDate;
use kabipay_db_entities::tenant::d0021_compensation::{compensation_review_cycle, salary_band};

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "SalaryBand")]
pub struct SalaryBandDto {
    pub id: ID,
    pub tenant_id: ID,
    pub designation_id: ID,
    pub grade: Option<i32>,
    pub min_salary: Option<String>,
    pub mid_salary: Option<String>,
    pub max_salary: Option<String>,
    pub currency: Option<String>,
    pub effective_year: Option<i32>,
}

impl From<salary_band::Model> for SalaryBandDto {
    fn from(m: salary_band::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            designation_id: ID(m.designation_id.to_string()),
            grade: m.grade,
            min_salary: m.min_salary.map(|d| d.to_string()),
            mid_salary: m.mid_salary.map(|d| d.to_string()),
            max_salary: m.max_salary.map(|d| d.to_string()),
            currency: m.currency,
            effective_year: m.effective_year,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "CompensationReviewCycle")]
pub struct CompensationReviewCycleDto {
    pub id: ID,
    pub tenant_id: ID,
    pub name: String,
    pub year: i32,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub status: String,
    pub budget_percentage: Option<String>,
}

impl From<compensation_review_cycle::Model> for CompensationReviewCycleDto {
    fn from(m: compensation_review_cycle::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            name: m.name,
            year: m.year,
            start_date: m.start_date,
            end_date: m.end_date,
            status: m.status,
            budget_percentage: m.budget_percentage.map(|d| d.to_string()),
        }
    }
}
