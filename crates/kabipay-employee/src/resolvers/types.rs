//! GraphQL output types for kabipay-employee.

use async_graphql::{InputObject, SimpleObject, ID};
use chrono::{DateTime, NaiveDate, Utc};

use crate::entities::d0006_org_hierarchy::{department, designation};
use crate::entities::d0007_employee_core::employee;
use crate::entities::d0008_document_system::{document_type, employee_document};
use crate::entities::d0017_onboarding_offboarding::onboarding_checklist;

/// Federated `Employee` type. `id` is the canonical cross-service identifier (Gap A).
#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Employee")]
pub struct EmployeeDto {
    pub id: ID,
    pub tenant_id: ID,
    pub employee_code: String,
    pub first_name: String,
    pub last_name: String,
    /// Computed convenience field: `first_name` + space + `last_name`.
    pub full_name: String,
    pub status: String,
    pub employment_type: Option<String>,
    pub date_of_joining: NaiveDate,
    pub department_id: Option<ID>,
    pub designation_id: Option<ID>,
    pub reporting_manager_id: Option<ID>,
    pub user_id: Option<ID>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "DocumentType")]
pub struct DocumentTypeDto {
    pub id: ID,
    pub tenant_id: ID,
    pub name: String,
    pub category: Option<String>,
    pub is_required: bool,
    pub expiry_alert_days: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<document_type::Model> for DocumentTypeDto {
    fn from(m: document_type::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            name: m.name,
            category: m.category,
            is_required: m.is_required,
            expiry_alert_days: m.expiry_alert_days,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "EmployeeDocument")]
pub struct EmployeeDocumentDto {
    pub id: ID,
    pub tenant_id: ID,
    pub employee_id: ID,
    pub document_type_id: ID,
    pub status: String,
    pub expiry_date: Option<NaiveDate>,
    pub uploaded_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<employee_document::Model> for EmployeeDocumentDto {
    fn from(m: employee_document::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            employee_id: ID(m.employee_id.to_string()),
            document_type_id: ID(m.document_type_id.to_string()),
            status: m.status,
            expiry_date: m.expiry_date,
            uploaded_at: m.uploaded_at,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "OnboardingChecklistItem")]
pub struct OnboardingChecklistItemDto {
    pub id: ID,
    pub tenant_id: ID,
    pub employee_id: ID,
    pub task_name: String,
    pub task_category: Option<String>,
    pub assigned_to: Option<ID>,
    pub is_completed: bool,
    pub due_date: Option<NaiveDate>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl From<onboarding_checklist::Model> for OnboardingChecklistItemDto {
    fn from(m: onboarding_checklist::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            employee_id: ID(m.employee_id.to_string()),
            task_name: m.task_name,
            task_category: m.task_category,
            assigned_to: m.assigned_to.map(|u| ID(u.to_string())),
            is_completed: m.is_completed,
            due_date: m.due_date,
            completed_at: m.completed_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Department")]
pub struct DepartmentDto {
    pub id: ID,
    pub tenant_id: ID,
    pub name: String,
    pub code: String,
    pub parent_department_id: Option<ID>,
}

impl From<department::Model> for DepartmentDto {
    fn from(m: department::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            name: m.name,
            code: m.code,
            parent_department_id: m.parent_department_id.map(|u| ID(u.to_string())),
        }
    }
}

/// Flat reporting-line row; clients build a tree from `reporting_manager_id`.
#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "OrgChartRow")]
pub struct OrgChartRowDto {
    pub employee_id: ID,
    pub employee_code: String,
    pub full_name: String,
    pub reporting_manager_id: Option<ID>,
    pub department_name: Option<String>,
    pub designation_title: Option<String>,
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Designation")]
pub struct DesignationDto {
    pub id: ID,
    pub tenant_id: ID,
    pub department_id: ID,
    pub title: String,
    pub level: Option<String>,
    pub grade: Option<i32>,
}

impl From<designation::Model> for DesignationDto {
    fn from(m: designation::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            department_id: ID(m.department_id.to_string()),
            title: m.title,
            level: m.level,
            grade: m.grade,
        }
    }
}

#[derive(InputObject, Clone, Debug)]
pub struct CreateEmployeeInput {
    pub employee_code: String,
    pub first_name: String,
    pub last_name: String,
    pub date_of_joining: NaiveDate,
    pub department_id: Option<ID>,
    pub designation_id: Option<ID>,
    /// Must be another active employee in the tenant; cannot be self (enforced after id is chosen).
    pub reporting_manager_id: Option<ID>,
    pub employment_type: Option<String>,
    /// Defaults to `ACTIVE` when omitted.
    pub status: Option<String>,
    pub user_id: Option<ID>,
}

#[derive(InputObject, Clone, Debug)]
pub struct UpdateEmployeeInput {
    pub id: ID,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub department_id: Option<ID>,
    pub designation_id: Option<ID>,
    /// Omitted = leave unchanged; `null` = clear manager; id = set manager (cycle-safe).
    pub reporting_manager_id: Option<Option<ID>>,
    pub employment_type: Option<String>,
    pub status: Option<String>,
    pub user_id: Option<ID>,
}

#[derive(InputObject, Clone, Debug)]
pub struct UploadEmployeeDocumentInput {
    pub employee_id: ID,
    pub document_type_id: ID,
    pub file_name: String,
    pub mime_type: Option<String>,
    /// Standard base64 (not data-URL). Max ~6MB decoded.
    pub content_base64: String,
}

impl From<employee::Model> for EmployeeDto {
    fn from(m: employee::Model) -> Self {
        let full_name = format!("{} {}", m.first_name.trim(), m.last_name.trim())
            .trim()
            .to_string();
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            employee_code: m.employee_code,
            first_name: m.first_name,
            last_name: m.last_name,
            full_name,
            status: m.status,
            employment_type: m.employment_type,
            date_of_joining: m.date_of_joining,
            department_id: m.department_id.map(|id| ID(id.to_string())),
            designation_id: m.designation_id.map(|id| ID(id.to_string())),
            reporting_manager_id: m.reporting_manager_id.map(|id| ID(id.to_string())),
            user_id: m.user_id.map(|id| ID(id.to_string())),
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}
