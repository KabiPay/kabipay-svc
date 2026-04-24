//! GraphQL DTOs for kabipay-attendance.

use async_graphql::{InputObject, SimpleObject, ID};
use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use kabipay_db_entities::tenant::d0010_time_shift_roster::{
    attendance, holiday, shift, timesheet_entry,
};

use crate::services::attendance_service::PunchDaySummary;

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Shift")]
pub struct ShiftDto {
    pub id: ID,
    pub tenant_id: ID,
    pub name: String,
    pub start_time: Option<NaiveTime>,
    pub end_time: Option<NaiveTime>,
    pub work_hours: Option<i32>,
    pub is_night_shift: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<shift::Model> for ShiftDto {
    fn from(m: shift::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            name: m.name,
            start_time: m.start_time,
            end_time: m.end_time,
            work_hours: m.work_hours,
            is_night_shift: m.is_night_shift,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Attendance")]
pub struct AttendanceDto {
    pub id: ID,
    pub tenant_id: ID,
    pub employee_id: ID,
    pub shift_id: Option<ID>,
    pub work_date: NaiveDate,
    pub check_in_time: Option<NaiveTime>,
    pub check_out_time: Option<NaiveTime>,
    /// WGS84 latitude for punch-in, when recorded (string decimal, matches DB `NUMERIC`).
    pub check_in_lat: Option<String>,
    pub check_in_lng: Option<String>,
    /// WGS84 coordinates for punch-out, when recorded.
    pub check_out_lat: Option<String>,
    pub check_out_lng: Option<String>,
    pub status: Option<String>,
    pub source: Option<String>,
    pub late_minutes: Option<i32>,
}

/// Optional client GPS (browser / mobile) for the **current** punch (in or out).
#[derive(InputObject, Clone, Debug)]
pub struct PunchTodayInput {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
}

/// Log a **completed** check-in and check-out for a **past or today** `workDate` when both
/// live punches were missed. Same calendar day only: check-in time must be before check-out.
#[derive(InputObject, Clone, Debug)]
pub struct AddManualAttendanceSegmentInput {
    pub work_date: NaiveDate,
    pub check_in_time: NaiveTime,
    pub check_out_time: NaiveTime,
}

/// One work day: all punch segments + sum of completed segment lengths (minutes).
#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "PunchDaySummary")]
pub struct PunchDaySummaryDto {
    pub work_date: NaiveDate,
    /// Sum of (check out − check in) for every **completed** segment that day.
    pub total_worked_minutes: i32,
    /// Current in-progress row (punched in, not out), if any.
    pub open_segment: Option<AttendanceDto>,
    /// All segment rows for that day, oldest first.
    pub segments: Vec<AttendanceDto>,
}

/// A holiday in a location calendar, with the parent calendar’s display name.
#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "HolidayEntry")]
pub struct HolidayEntryDto {
    pub id: ID,
    pub calendar_id: ID,
    pub calendar_name: String,
    pub holiday_date: NaiveDate,
    pub name: String,
    /// Optional category, e.g. NATIONAL, REGIONAL
    pub holiday_type: Option<String>,
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "TimesheetEntry")]
pub struct TimesheetEntryDto {
    pub id: ID,
    pub tenant_id: ID,
    pub employee_id: ID,
    pub work_date: NaiveDate,
    pub hours_worked: String,
    pub project_code: Option<String>,
    pub description: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<timesheet_entry::Model> for TimesheetEntryDto {
    fn from(m: timesheet_entry::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            employee_id: ID(m.employee_id.to_string()),
            work_date: m.work_date,
            hours_worked: m.hours_worked.to_string(),
            project_code: m.project_code,
            description: m.description,
            status: m.status,
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

#[derive(InputObject, Clone, Debug)]
pub struct CreateTimesheetEntryInput {
    pub work_date: NaiveDate,
    pub hours_worked: String,
    pub project_code: Option<String>,
    pub description: Option<String>,
}

impl HolidayEntryDto {
    pub fn from_holiday(m: holiday::Model, calendar_name: String) -> Self {
        Self {
            id: ID(m.id.to_string()),
            calendar_id: ID(m.calendar_id.to_string()),
            calendar_name,
            holiday_date: m.holiday_date,
            name: m.name,
            holiday_type: m.r#type,
        }
    }
}

impl From<attendance::Model> for AttendanceDto {
    fn from(m: attendance::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            employee_id: ID(m.employee_id.to_string()),
            shift_id: m.shift_id.map(|id| ID(id.to_string())),
            work_date: m.work_date,
            check_in_time: m.check_in_time,
            check_out_time: m.check_out_time,
            check_in_lat: m.check_in_lat.map(|d| d.to_string()),
            check_in_lng: m.check_in_lng.map(|d| d.to_string()),
            check_out_lat: m.check_out_lat.map(|d| d.to_string()),
            check_out_lng: m.check_out_lng.map(|d| d.to_string()),
            status: m.status,
            source: m.source,
            late_minutes: m.late_minutes,
        }
    }
}

impl From<PunchDaySummary> for PunchDaySummaryDto {
    fn from(s: PunchDaySummary) -> Self {
        Self {
            work_date: s.work_date,
            total_worked_minutes: s.total_worked_minutes,
            open_segment: s.open_segment.map(AttendanceDto::from),
            segments: s.segments.into_iter().map(AttendanceDto::from).collect(),
        }
    }
}
