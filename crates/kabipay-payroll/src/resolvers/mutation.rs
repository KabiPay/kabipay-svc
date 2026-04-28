//! Write operations: v1 pay run.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    subgraph::{require_client_claims, require_tenant_id, tenant_db},
    KabiPayError,
};

use rust_decimal::Decimal;
use std::str::FromStr;

use crate::resolvers::types::{
    CreatePayrollArrearInput, CreatePayrollCycleInput, PayrollArrearDto, PayrollComplianceSettingDto,
    PayrollCycleDto, UpsertPayrollComplianceSettingInput,
};
use crate::services::arrear_service;
use crate::services::payroll_service;
use crate::resolvers::query::parse_uuid;

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Record a **PENDING** arrear for an employee; amount is added on the next pay run (with an `ARREAR` line).
    async fn create_payroll_arrear(
        &self,
        ctx: &Context<'_>,
        input: CreatePayrollArrearInput,
    ) -> Result<PayrollArrearDto> {
        let claims = require_client_claims(ctx)?;
        if !claims.can_export_payroll_statutory() {
            return Err(
                KabiPayError::Forbidden(
                    "create payroll arrear requires payroll:statutory_export or HR / tenant admin"
                        .into(),
                )
                .into_graphql(),
            );
        }
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let eid = parse_uuid(&input.employee_id, "employeeId")?;
        let amount = Decimal::from_str(&input.amount.trim())
            .map_err(|e| KabiPayError::Validation(format!("amount: {e}")).into_graphql())?;
        let m = arrear_service::create_arrear(&db, tenant_id, eid, amount, input.reason)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(PayrollArrearDto::from(m))
    }

    /// Create a **DRAFT** payroll cycle for a calendar month/year (one per tenant per period in v1).
    /// Same RBAC as **run payroll** (statutory export / HR / tenant admin).
    async fn create_payroll_cycle(
        &self,
        ctx: &Context<'_>,
        input: CreatePayrollCycleInput,
    ) -> Result<PayrollCycleDto> {
        let claims = require_client_claims(ctx)?;
        if !claims.can_export_payroll_statutory() {
            return Err(
                KabiPayError::Forbidden(
                    "create payroll cycle requires payroll:statutory_export or HR / tenant admin role"
                        .into(),
                )
                .into_graphql(),
            );
        }
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let m = payroll_service::create_payroll_cycle(
            &db,
            tenant_id,
            input.name,
            input.month,
            input.year,
            input.payment_date,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(PayrollCycleDto::from(m))
    }

    /// Upsert tenant employer TAN and legal name for India statutory payroll CSV placeholders.
    async fn upsert_payroll_compliance_setting(
        &self,
        ctx: &Context<'_>,
        input: UpsertPayrollComplianceSettingInput,
    ) -> Result<PayrollComplianceSettingDto> {
        let claims = require_client_claims(ctx)?;
        if !claims.can_export_payroll_statutory() {
            return Err(
                KabiPayError::Forbidden(
                    "upsert payroll compliance setting requires payroll:statutory_export or HR / tenant admin role"
                        .into(),
                )
                .into_graphql(),
            );
        }
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let logo = input
            .payslip_logo_file_storage_id
            .as_ref()
            .map(|id| parse_uuid(id, "payslipLogoFileStorageId"))
            .transpose()?;
        let m = payroll_service::upsert_payroll_compliance_setting(
            &db,
            tenant_id,
            input.employer_tan,
            input.employer_legal_name,
            input.base_salary_component_code,
            input.arrear_salary_component_code,
            input.payslip_header_title,
            logo,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(PayrollComplianceSettingDto::from(m))
    }

    /// **Pay run (v2)** — generate missing payslips for a `DRAFT` cycle, then set the cycle to
    /// `PROCESSED`. Per employee: latest `employment_history.salary` as BASIC, PENDING
    /// `payroll_arrear` as an `ARREAR` `salary_component` line, India statutory stub and TDS from
    /// `tax_computation` for the pay month’s India FY. Same RBAC as India statutory CSV export.
    async fn run_payroll_for_cycle(
        &self,
        ctx: &Context<'_>,
        payroll_cycle_id: ID,
    ) -> Result<PayrollCycleDto> {
        let claims = require_client_claims(ctx)?;
        if !claims.can_export_payroll_statutory() {
            return Err(
                KabiPayError::Forbidden(
                    "run payroll requires payroll:statutory_export or HR / tenant admin role"
                        .into(),
                )
                .into_graphql(),
            );
        }
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let cid = parse_uuid(&payroll_cycle_id, "payrollCycleId")?;
        let m = payroll_service::run_payroll_for_cycle(
            &db,
            tenant_id,
            cid,
            claims.sub,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(PayrollCycleDto::from(m))
    }
}
