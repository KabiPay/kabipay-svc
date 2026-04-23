//! GraphQL DTOs for kabipay-recruitment.

use async_graphql::{SimpleObject, ID};
use chrono::{DateTime, NaiveDate, Utc};
use kabipay_db_entities::tenant::d0016_recruitment::{application, job_posting};

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "JobPosting")]
pub struct JobPostingDto {
    pub id: ID,
    pub tenant_id: ID,
    pub title: String,
    pub description: Option<String>,
    pub employment_type: Option<String>,
    pub vacancies: i32,
    pub status: String,
    pub open_date: Option<NaiveDate>,
    pub close_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<job_posting::Model> for JobPostingDto {
    fn from(m: job_posting::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            title: m.title,
            description: m.description,
            employment_type: m.employment_type,
            vacancies: m.vacancies,
            status: m.status,
            open_date: m.open_date,
            close_date: m.close_date,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Application")]
pub struct ApplicationDto {
    pub id: ID,
    pub tenant_id: ID,
    pub job_id: ID,
    pub candidate_name: String,
    pub candidate_email: String,
    pub candidate_phone: Option<String>,
    pub source: Option<String>,
    pub status: String,
    pub applied_at: DateTime<Utc>,
}

impl From<application::Model> for ApplicationDto {
    fn from(m: application::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            job_id: ID(m.job_id.to_string()),
            candidate_name: m.candidate_name,
            candidate_email: m.candidate_email,
            candidate_phone: m.candidate_phone,
            source: m.source,
            status: m.status,
            applied_at: m.applied_at,
        }
    }
}
