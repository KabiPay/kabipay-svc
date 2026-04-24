//! Write operations for attendance (punch in / out) and timesheet entries.

use async_graphql::{Context, Object, Result, ID};
use kabipay_common::{
    subgraph::{
        client_request_hints, require_client_claims, require_tenant_id, resolve_client_employee_id,
        tenant_db,
    },
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::types::{
    AddManualAttendanceSegmentInput, AttendanceDto, AttendancePunchPolicyDto,
    CreateTimesheetEntryInput, PunchTodayInput, TimesheetEntryDto,
    UpsertAttendancePunchPolicyInput,
};
use crate::services::{attendance_service, punch_policy};

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
    ///
    /// When `input` includes **both** `latitude` and `longitude` (WGS84), they are stored on
    /// `attendance` as punch-in coordinates for a new row, or punch-out coordinates when closing
    /// an open segment (`check_out_lat` / `check_out_lng` columns).
    async fn punch_today(
        &self,
        ctx: &Context<'_>,
        input: Option<PunchTodayInput>,
    ) -> Result<AttendanceDto> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let employee_id = resolve_client_employee_id(ctx, &db, tenant_id)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let geo = match input {
            None => None,
            Some(i) => attendance_service::parse_punch_geo(i.latitude, i.longitude)
                .map_err(KabiPayError::into_graphql)?,
        };
        let hints = client_request_hints(ctx);
        let client_ip = hints.client_ip.as_deref();
        let m = attendance_service::punch_today(&db, tenant_id, employee_id, geo, client_ip)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(AttendanceDto::from(m))
    }

    /// Create or update the tenant’s live punch policy (geofence + IP allowlist).
    async fn upsert_attendance_punch_policy(
        &self,
        ctx: &Context<'_>,
        input: UpsertAttendancePunchPolicyInput,
    ) -> Result<AttendancePunchPolicyDto> {
        let tenant_id = require_tenant_id(ctx)?;
        let claims = require_client_claims(ctx)?;
        if !claims.can_configure_attendance_punch_policy() {
            return Err(KabiPayError::Forbidden(
                "attendance punch policy is restricted to HR / tenant admins".into(),
            )
            .into_graphql());
        }
        let db = tenant_db(ctx, tenant_id).await?;
        let m = punch_policy::upsert_punch_policy(
            &db,
            tenant_id,
            input.is_enforced,
            input.site_latitude,
            input.site_longitude,
            input.max_distance_meters,
            input.ip_allowlist,
        )
        .await
        .map_err(KabiPayError::into_graphql)?;
        Ok(AttendancePunchPolicyDto::from(m))
    }

    /// Add a full **in + out** segment for a `workDate` (no future dates) when the user did not
    /// punch live — does not modify `punch_today` behaviour.
    async fn add_manual_attendance_segment(
        &self,
        ctx: &Context<'_>,
        input: AddManualAttendanceSegmentInput,
    ) -> Result<AttendanceDto> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let employee_id = resolve_client_employee_id(ctx, &db, tenant_id)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let m = attendance_service::add_manual_attendance_segment(
            &db,
            tenant_id,
            employee_id,
            input.work_date,
            input.check_in_time,
            input.check_out_time,
        )
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
        let h = attendance_service::parse_hours(&input.hours_worked)
            .map_err(KabiPayError::into_graphql)?;
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
