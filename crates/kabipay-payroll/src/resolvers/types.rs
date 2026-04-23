//! GraphQL DTOs for kabipay-payroll.

use async_graphql::{SimpleObject, ID};
use chrono::{DateTime, NaiveDate, Utc};
use kabipay_db_entities::tenant::d0012_payroll::{payroll_cycle, payslip, payslip_component, salary_component};

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "SalaryComponent")]
pub struct SalaryComponentDto {
    pub id: ID,
    pub tenant_id: ID,
    pub name: String,
    pub code: String,
    pub component_type: String,
    pub is_taxable: bool,
    pub is_fixed: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<salary_component::Model> for SalaryComponentDto {
    fn from(m: salary_component::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            name: m.name,
            code: m.code,
            component_type: m.r#type,
            is_taxable: m.is_taxable,
            is_fixed: m.is_fixed,
            is_active: m.is_active,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "PayrollCycle")]
pub struct PayrollCycleDto {
    pub id: ID,
    pub tenant_id: ID,
    pub name: String,
    pub month: i32,
    pub year: i32,
    pub status: String,
    pub payment_date: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "PayslipComponentLine")]
pub struct PayslipComponentLineDto {
    pub id: ID,
    pub tenant_id: ID,
    pub payslip_id: ID,
    pub salary_component_id: ID,
    /// Decimal as string
    pub amount: String,
    pub component_type: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<payslip_component::Model> for PayslipComponentLineDto {
    fn from(m: payslip_component::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            payslip_id: ID(m.payslip_id.to_string()),
            salary_component_id: ID(m.salary_component_id.to_string()),
            amount: m.amount.to_string(),
            component_type: m.component_type,
            created_at: m.created_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Payslip")]
pub struct PayslipDetailDto {
    pub id: ID,
    pub tenant_id: ID,
    pub employee_id: ID,
    pub payroll_cycle_id: ID,
    pub gross_salary: String,
    pub total_deductions: String,
    pub net_salary: String,
    pub pf_employee: Option<String>,
    pub pf_employer: Option<String>,
    pub esi_employee: Option<String>,
    pub esi_employer: Option<String>,
    pub tds_amount: Option<String>,
    pub professional_tax: Option<String>,
    pub uan_number: Option<String>,
    pub esic_number: Option<String>,
    pub status: String,
    pub generated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub lines: Vec<PayslipComponentLineDto>,
}

impl PayslipDetailDto {
    pub fn from_head(m: payslip::Model, lines: Vec<payslip_component::Model>) -> Self {
        let lines = lines.into_iter().map(PayslipComponentLineDto::from).collect();
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            employee_id: ID(m.employee_id.to_string()),
            payroll_cycle_id: ID(m.payroll_cycle_id.to_string()),
            gross_salary: m.gross_salary.to_string(),
            total_deductions: m.total_deductions.to_string(),
            net_salary: m.net_salary.to_string(),
            pf_employee: m.pf_employee.map(|d| d.to_string()),
            pf_employer: m.pf_employer.map(|d| d.to_string()),
            esi_employee: m.esi_employee.map(|d| d.to_string()),
            esi_employer: m.esi_employer.map(|d| d.to_string()),
            tds_amount: m.tds_amount.map(|d| d.to_string()),
            professional_tax: m.professional_tax.map(|d| d.to_string()),
            uan_number: m.uan_number,
            esic_number: m.esic_number,
            status: m.status,
            generated_at: m.generated_at,
            created_at: m.created_at,
            updated_at: m.updated_at,
            lines,
        }
    }
}

impl From<payroll_cycle::Model> for PayrollCycleDto {
    fn from(m: payroll_cycle::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            name: m.name,
            month: m.month,
            year: m.year,
            status: m.status,
            payment_date: m.payment_date,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}
