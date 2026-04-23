//! Root query resolvers for kabipay-payroll.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    subgraph::{require_tenant_id, resolve_client_employee_id, tenant_db},
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::types::{PayslipDetailDto, PayrollCycleDto, SalaryComponentDto};
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
    async fn payslip(
        &self,
        ctx: &Context<'_>,
        id: ID,
    ) -> Result<Option<PayslipDetailDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let sid = parse_uuid(&id, "id")?;
        let row = payroll_service::find_payslip_detail(&db, tenant_id, sid)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(row.map(|(p, c)| PayslipDetailDto::from_head(p, c)))
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
}
