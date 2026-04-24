//! Root query resolvers for kabipay-payroll.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    client_data_scope::{
        data_scope_from_context, resolve_employee_scope_filter, resolve_viewer_employee,
    },
    context::SCOPE_RES_EMPLOYEE,
    subgraph::{require_client_claims, require_tenant_id, resolve_client_employee_id, tenant_db},
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::types::{PayrollCycleDto, PayslipDetailDto, SalaryComponentDto};
use crate::services::payroll_service;

fn parse_uuid(id: &ID, field: &'static str) -> Result<Uuid> {
    Uuid::parse_str(id.as_str())
        .map_err(|e| KabiPayError::Validation(format!("invalid {field}: {e}")).into_graphql())
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn payroll_health(&self) -> &'static str {
        "ok"
    }

    /// List salary components (earnings/deductions) for the caller's tenant.
    async fn salary_components(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = true)] active_only: bool,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<SalaryComponentDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = payroll_service::list_components(&db, tenant_id, active_only, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(SalaryComponentDto::from).collect())
    }

    /// List payroll cycles for the caller's tenant, most recent first.
    async fn payroll_cycles(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 24)] limit: u64,
    ) -> Result<Vec<PayrollCycleDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = payroll_service::list_cycles(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(PayrollCycleDto::from).collect())
    }

    /// One payslip with `lines` = `payslip_component` rows.
    async fn payslip(&self, ctx: &Context<'_>, id: ID) -> Result<Option<PayslipDetailDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let sid = parse_uuid(&id, "id")?;
        let row = payroll_service::find_payslip_detail(&db, tenant_id, sid)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let Some((p, c)) = row else {
            return Ok(None);
        };
        let scope = data_scope_from_context(ctx, SCOPE_RES_EMPLOYEE);
        let viewer = resolve_viewer_employee(ctx, &db, tenant_id).await?;
        let filt = resolve_employee_scope_filter(&db, tenant_id, scope, viewer)
            .await
            .map_err(KabiPayError::into_graphql)?;
        if !filt.allows_employee(p.employee_id) {
            return Ok(None);
        }
        Ok(Some(PayslipDetailDto::from_head(p, c)))
    }

    /// When `employeeId` is omitted, uses the signed-in user’s employee id from the JWT
    /// (or `user` → `employee` link). Pass `employeeId` to view a specific person (e.g. HR).
    async fn payslips(
        &self,
        ctx: &Context<'_>,
        employee_id: Option<ID>,
        #[graphql(default = 24)] limit: u64,
    ) -> Result<Vec<PayslipDetailDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let empl = if let Some(id) = &employee_id {
            parse_uuid(id, "employeeId")?
        } else {
            resolve_client_employee_id(ctx, &db, tenant_id)
                .await
                .map_err(KabiPayError::into_graphql)?
        };
        let scope = data_scope_from_context(ctx, SCOPE_RES_EMPLOYEE);
        let viewer = resolve_viewer_employee(ctx, &db, tenant_id).await?;
        let filt = resolve_employee_scope_filter(&db, tenant_id, scope, viewer)
            .await
            .map_err(KabiPayError::into_graphql)?;
        if !filt.allows_employee(empl) {
            return Ok(vec![]);
        }
        let list = payroll_service::list_payslips(&db, tenant_id, Some(empl), limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let ids: Vec<Uuid> = list.iter().map(|p| p.id).collect();
        let lines = payroll_service::payslip_lines_by_payslip_ids(&db, tenant_id, &ids)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let out: Vec<PayslipDetailDto> = list
            .into_iter()
            .map(|p| {
                let c = lines.get(&p.id).cloned().unwrap_or_default();
                PayslipDetailDto::from_head(p, c)
            })
            .collect();
        Ok(out)
    }

    /// **India — monthly TDS summary (CSV).** All payslips for the payroll cycle matching
    /// `month` + `calendar year`. Requires `payroll:statutory_export` or HR / tenant admin role.
    /// Stub for statutory filing prep — not a filed Form 24Q; values come from `payslip.tds_amount`.
    async fn india_tds_monthly_summary_csv(
        &self,
        ctx: &Context<'_>,
        month: i32,
        year: i32,
    ) -> Result<String> {
        let tenant_id = require_tenant_id(ctx)?;
        let claims = require_client_claims(ctx)?;
        if !claims.can_export_payroll_statutory() {
            return Err(
                KabiPayError::Forbidden(
                    "payroll statutory export requires payroll:statutory_export or HR / tenant admin role"
                        .into(),
                )
                .into_graphql(),
            );
        }
        let db = tenant_db(ctx, tenant_id).await?;
        payroll_service::india_tds_monthly_summary_csv(&db, tenant_id, month, year)
            .await
            .map_err(KabiPayError::into_graphql)
    }

    /// **India — monthly PF / ESI summary (CSV).** Payslip statutory columns (`pfEmployee`, `esiEmployee`, UAN, ESIC, …)
    /// for every payslip in the payroll cycle matching `month` + `year`. Same RBAC as TDS export; not ECR / challan output.
    async fn india_pf_esi_monthly_summary_csv(
        &self,
        ctx: &Context<'_>,
        month: i32,
        year: i32,
    ) -> Result<String> {
        let tenant_id = require_tenant_id(ctx)?;
        let claims = require_client_claims(ctx)?;
        if !claims.can_export_payroll_statutory() {
            return Err(
                KabiPayError::Forbidden(
                    "payroll statutory export requires payroll:statutory_export or HR / tenant admin role"
                        .into(),
                )
                .into_graphql(),
            );
        }
        let db = tenant_db(ctx, tenant_id).await?;
        payroll_service::india_pf_esi_monthly_summary_csv(&db, tenant_id, month, year)
            .await
            .map_err(KabiPayError::into_graphql)
    }
}
