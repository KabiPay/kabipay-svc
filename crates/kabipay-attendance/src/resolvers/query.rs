//! Root query resolvers for kabipay-attendance.

use async_graphql::{Context, Object, Result, ID};
use chrono::{NaiveDate, Utc};
use kabipay_common::{
    subgraph::{require_tenant_id, resolve_client_employee_id, tenant_db},
    KabiPayError,
};
use uuid::Uuid;

use crate::resolvers::types::{
    AttendanceDto, HolidayEntryDto, PunchDaySummaryDto, ShiftDto, TimesheetEntryDto,
};
use crate::services::attendance_service;

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    async fn attendance_health(&self) -> &'static str {
        "ok"
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
        let rows = attendance_service::list_attendance(&db, tenant_id, limit)
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
        let rows = attendance_service::list_timesheet_entries(&db, tenant_id, emp, limit)
            .await
            .map_err(KabiPayError::into_graphql)?;
        Ok(rows.into_iter().map(TimesheetEntryDto::from).collect())
    }
}

fn parse_uuid(id: &ID, field: &'static str) -> Result<Uuid> {
    Uuid::parse_str(id.as_str())
        .map_err(|e| KabiPayError::Validation(format!("invalid {field}: {e}")).into_graphql())
}
