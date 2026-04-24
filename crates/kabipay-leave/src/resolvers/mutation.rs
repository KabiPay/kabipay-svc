//! Write operations for the leave domain.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    subgraph::{require_client_claims, require_tenant_id, resolve_client_employee_id, tenant_db},
    KabiPayError,
};

use crate::resolvers::query::parse_uuid;
use crate::resolvers::types::{LeaveRequestDto, SubmitLeaveRequestInput};
use crate::services::leave_service;

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Create a PENDING leave request and reserve days against the annual balance.
    async fn submit_leave_request(
        &self,
        ctx: &Context<'_>,
        input: SubmitLeaveRequestInput,
    ) -> Result<LeaveRequestDto> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let employee_id = resolve_client_employee_id(ctx, &db, tenant_id)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let leave_type_id = parse_uuid(&input.leave_type_id, "leaveTypeId")?;
        let m = leave_service::submit_leave_request(
            &db,
            tenant_id,
            employee_id,
            leave_type_id,
            input.from_date,
            input.to_date,
            input.is_half_day,
            input.half_day_session,
            input.reason,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(LeaveRequestDto::from(m))
    }

    /// Set a PENDING request to APPROVED and credit used leave (see `submit_leave_request` balance flow).
    async fn approve_leave_request(
        &self,
        ctx: &Context<'_>,
        leave_request_id: ID,
    ) -> Result<LeaveRequestDto> {
        let claims = require_client_claims(ctx)?;
        if !claims.can_approve_leave() {
            return Err(KabiPayError::Forbidden(
                "leave approve permission required (leave:approve or HR/tenant admin role)".into(),
            )
            .into_graphql());
        }
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let id = parse_uuid(&leave_request_id, "leaveRequestId")?;
        let m = leave_service::approve_leave_request(&db, tenant_id, id, claims.sub)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(LeaveRequestDto::from(m))
    }

    /// Reject a PENDING request and release the balance reservation.
    async fn reject_leave_request(
        &self,
        ctx: &Context<'_>,
        leave_request_id: ID,
        reason: Option<String>,
    ) -> Result<LeaveRequestDto> {
        let claims = require_client_claims(ctx)?;
        if !claims.can_approve_leave() {
            return Err(KabiPayError::Forbidden(
                "leave approve permission required (leave:approve or HR/tenant admin role)".into(),
            )
            .into_graphql());
        }
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let id = parse_uuid(&leave_request_id, "leaveRequestId")?;
        let m = leave_service::reject_leave_request(&db, tenant_id, id, claims.sub, reason)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(LeaveRequestDto::from(m))
    }
}
