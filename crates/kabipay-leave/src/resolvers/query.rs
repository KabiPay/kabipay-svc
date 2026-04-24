//! Root query resolvers for kabipay-leave.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::client_data_scope::{
    data_scope_from_context, resolve_employee_scope_filter, resolve_viewer_employee,
};
use kabipay_common::context::SCOPE_RES_LEAVE;
use kabipay_common::{
    subgraph::{require_tenant_id, resolve_client_employee_id, tenant_db},
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::types::{LeaveBalanceDto, LeaveRequestDto, LeaveTypeDto};
use crate::services::leave_service;

pub(crate) fn parse_uuid(raw: &ID, field: &'static str) -> Result<Uuid> {
    Uuid::parse_str(raw.as_str())
        .map_err(|e| KabiPayError::Validation(format!("invalid {field}: {e}")).into_graphql())
}

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn leave_health(&self) -> &'static str {
        "ok"
    }

    /// List leave types for the caller's tenant.
    async fn leave_types(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<LeaveTypeDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = leave_service::list_types(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(LeaveTypeDto::from).collect())
    }

    /// List leave requests for the caller's tenant.
    async fn leave_requests(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<LeaveRequestDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let scope = data_scope_from_context(ctx, SCOPE_RES_LEAVE);
        let viewer = resolve_viewer_employee(ctx, &db, tenant_id).await?;
        let rows = leave_service::list_requests(&db, tenant_id, limit, scope, viewer)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(LeaveRequestDto::from).collect())
    }

    /// Leave-balance rows for an employee. Pass `employeeId` to target a
    /// specific person (e.g. HR view); when omitted, the caller's own
    /// employee id is resolved from the JWT (requires `Authorization`).
    async fn leave_balances(
        &self,
        ctx: &Context<'_>,
        employee_id: Option<ID>,
        year: Option<i32>,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<LeaveBalanceDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let emp = match &employee_id {
            Some(id) => parse_uuid(id, "employeeId")?,
            None => resolve_client_employee_id(ctx, &db, tenant_id)
                .await
                .map_err(KabiPayError::into_graphql)?,
        };
        let scope = data_scope_from_context(ctx, SCOPE_RES_LEAVE);
        let viewer = resolve_viewer_employee(ctx, &db, tenant_id).await?;
        let filt = resolve_employee_scope_filter(&db, tenant_id, scope, viewer)
            .await
            .map_err(KabiPayError::into_graphql)?;
        if !filt.allows_employee(emp) {
            return Ok(vec![]);
        }
        let rows = leave_service::list_balances_for_employee(&db, tenant_id, emp, year, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(LeaveBalanceDto::from).collect())
    }
}
