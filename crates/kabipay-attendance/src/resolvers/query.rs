//! Root query resolvers for kabipay-attendance.

use async_graphql::{Context, Object, Result, ID};
use chrono::{NaiveDate, Utc};
use kabipay_common::{
    client_data_scope::{
        data_scope_from_context, resolve_employee_scope_filter, resolve_viewer_employee,
    },
    context::SCOPE_RES_ATTENDANCE,
    subgraph::{require_client_claims, require_tenant_id, resolve_client_employee_id, tenant_db},
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::types::{
    AttendanceDto, AttendancePunchPolicyDto, HolidayCalendarDto, HolidayDayDto, HolidayEntryDto,
    PunchDaySummaryDto, ShiftDto, TimesheetEntryDto,
};
use crate::services::{attendance_service, punch_policy};

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn attendance_health(&self) -> &'static str {
        "ok"
    }

    /// Live punch policy (geofence + IP). **HR / tenant admin only** — not exposed to every employee.
    async fn attendance_punch_policy(&self, ctx: &Context<'_>) -> Result<AttendancePunchPolicyDto> {
        let tenant_id = require_tenant_id(ctx)?;
        let claims = require_client_claims(ctx)?;
        if !claims.can_configure_attendance_punch_policy() {
            return Err(KabiPayError::Forbidden(
                "attendance punch policy is restricted to HR / tenant admins".into(),
            )
            .into_graphql());
        }
        let db = tenant_db(ctx, tenant_id).await?;
        let row = punch_policy::find_punch_policy(&db, tenant_id)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(match row {
            Some(m) => AttendancePunchPolicyDto::from(m),
            None => AttendancePunchPolicyDto::not_configured(tenant_id),
        })
    }

    /// List all shift templates for the caller's tenant.
    async fn shifts(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<ShiftDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = attendance_service::list_shifts(&db, tenant_id, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(ShiftDto::from).collect())
    }

    /// Recent attendance rows for the caller's tenant, newest first.
    async fn attendance(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<AttendanceDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let scope = data_scope_from_context(ctx, SCOPE_RES_ATTENDANCE);
        let viewer = resolve_viewer_employee(ctx, &db, tenant_id).await?;
        let filt = resolve_employee_scope_filter(&db, tenant_id, scope, viewer)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let rows = attendance_service::list_attendance(&db, tenant_id, limit, &filt)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(AttendanceDto::from).collect())
    }

    /// Multi-segment punch for one work day: total worked minutes and all segments
    /// (JWT employee; `workDate` defaults to today, UTC `work_date` calendar).
    async fn punch_day_summary(
        &self,
        ctx: &Context<'_>,
        work_date: Option<NaiveDate>,
    ) -> Result<PunchDaySummaryDto> {
        let tenant_id = require_tenant_id(ctx)?;
        let claims = require_client_claims(ctx)?;
        if !claims.can_record_own_attendance_punches() {
            return Err(
                KabiPayError::Forbidden(
                    "attendance:punch_self or employee directory permission required".into(),
                )
                .into_graphql(),
            );
        }
        let db = tenant_db(ctx, tenant_id).await?;
        let employee_id = resolve_client_employee_id(ctx, &db, tenant_id)
            .await
            .map_err(KabiPayError::into_graphql)?;
        let date = work_date.unwrap_or_else(|| Utc::now().date_naive());
        let s = attendance_service::punch_day_summary(&db, tenant_id, employee_id, date)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(s.into())
    }

    /// Holidays on or after `fromDate` (defaults to today), all calendars in the tenant.
    async fn upcoming_holidays(
        &self,
        ctx: &Context<'_>,
        from_date: Option<NaiveDate>,
        #[graphql(default = 30)] limit: u64,
    ) -> Result<Vec<HolidayEntryDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let from = from_date.unwrap_or_else(|| Utc::now().naive_utc().date());
        let rows = attendance_service::list_upcoming_holidays(&db, tenant_id, from, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows
            .into_iter()
            .map(|(h, n)| HolidayEntryDto::from_holiday(h, n))
            .collect())
    }

    /// Timesheet rows for an employee. Omit `employeeId` to use the JWT-linked employee.
    async fn timesheet_entries(
        &self,
        ctx: &Context<'_>,
        employee_id: Option<ID>,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<TimesheetEntryDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let db = tenant_db(ctx, tenant_id).await?;
        let emp = if let Some(id) = &employee_id {
            parse_uuid(id, "employeeId")?
        } else {
            resolve_client_employee_id(ctx, &db, tenant_id)
                .await
                .map_err(KabiPayError::into_graphql)?
        };
        let scope = data_scope_from_context(ctx, SCOPE_RES_ATTENDANCE);
        let viewer = resolve_viewer_employee(ctx, &db, tenant_id).await?;
        let filt = resolve_employee_scope_filter(&db, tenant_id, scope, viewer)
            .await
            .map_err(KabiPayError::into_graphql)?;
        if !filt.allows_employee(emp) {
            return Ok(vec![]);
        }
        let rows = attendance_service::list_timesheet_entries(&db, tenant_id, emp, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(TimesheetEntryDto::from).collect())
    }

    /// Admin: list holiday calendars (tenant). Requires leave configuration permission.
    async fn holiday_calendars(
        &self,
        ctx: &Context<'_>,
        year: Option<i32>,
        #[graphql(default = 50)] limit: u64,
    ) -> Result<Vec<HolidayCalendarDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let claims = require_client_claims(ctx)?;
        if !claims.can_manage_leave_configuration() {
            return Err(
                KabiPayError::Forbidden("holiday calendar admin requires leave configuration permission".into())
                    .into_graphql(),
            );
        }
        let db = tenant_db(ctx, tenant_id).await?;
        let rows = attendance_service::list_holiday_calendars(&db, tenant_id, year, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(HolidayCalendarDto::from).collect())
    }

    /// Admin: holidays in a calendar. Requires leave configuration permission.
    async fn holidays_in_calendar(
        &self,
        ctx: &Context<'_>,
        calendar_id: ID,
        #[graphql(default = 100)] limit: u64,
    ) -> Result<Vec<HolidayDayDto>> {
        let tenant_id = require_tenant_id(ctx)?;
        let claims = require_client_claims(ctx)?;
        if !claims.can_manage_leave_configuration() {
            return Err(
                KabiPayError::Forbidden("holiday admin requires leave configuration permission".into())
                    .into_graphql(),
            );
        }
        let db = tenant_db(ctx, tenant_id).await?;
        let cid = parse_uuid(&calendar_id, "calendarId")?;
        let rows =
            attendance_service::list_holidays_in_calendar(&db, tenant_id, cid, limit)
                .await
                .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(HolidayDayDto::from).collect())
    }
}

fn parse_uuid(id: &ID, field: &'static str) -> Result<Uuid> {
    Uuid::parse_str(id.as_str())
        .map_err(|e| KabiPayError::Validation(format!("invalid {field}: {e}")).into_graphql())
}
