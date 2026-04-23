//! Write operations for the leave domain.

use async_graphql::{Context, Object, Result};
use kabipay_common::{
    subgraph::{require_tenant_id, resolve_client_employee_id, tenant_db},
    KabiPayError,
};

use crate::resolvers::types::{LeaveRequestDto, SubmitLeaveRequestInput};
use crate::services::leave_service;
use crate::resolvers::query::parse_uuid;

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
}
