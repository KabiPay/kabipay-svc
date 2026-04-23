//! Write operations for attendance (punch in / out) and timesheet entries.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    subgraph::{require_tenant_id, resolve_client_employee_id, tenant_db},
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::types::{AttendanceDto, CreateTimesheetEntryInput, TimesheetEntryDto};
use crate::services::attendance_service;

fn parse_uuid(id: &ID, field: &'static str) -> Result<Uuid> {
    Uuid::parse_str(id.as_str())
        .map_err(|e| KabiPayError::Validation(format!("invalid {field}: {e}")).into_graphql())
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Record a punch: closes the **open** segment (punch in without out) if any, otherwise
    /// starts a **new** segment (new `attendance` row). Multiple in/out pairs per `work_date`
    /// are allowed; there is no “third punch” error.
    async fn punch_today(&self, ctx: &Context<'_>) -> Result<AttendanceDto> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let employee_id = resolve_client_employee_id(ctx, &db, tenant_id)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let m = attendance_service::punch_today(&db, tenant_id, employee_id)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(AttendanceDto::from(m))
    }

    async fn create_timesheet_entry(
        &self,
        ctx: &Context<'_>,
        input: CreateTimesheetEntryInput,
    ) -> Result<TimesheetEntryDto> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let employee_id = resolve_client_employee_id(ctx, &db, tenant_id)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let h = attendance_service::parse_hours(&input.hours_worked).map_err(KabiPayError::into_graphql)?;
        let m = attendance_service::create_timesheet_entry(
            &db,
            tenant_id,
            employee_id,
            input.work_date,
            h,
            input.project_code,
            input.description,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(TimesheetEntryDto::from(m))
    }

    /// Soft-deletes a row; it must belong to the caller’s employee.
    async fn delete_timesheet_entry(&self, ctx: &Context<'_>, id: ID) -> Result<bool> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let employee_id = resolve_client_employee_id(ctx, &db, tenant_id)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let eid = parse_uuid(&id, "id")?;
        attendance_service::delete_timesheet_entry(&db, tenant_id, employee_id, eid)
            .await
            .map_err(KabiPayError::into_graphql)
    }
}
