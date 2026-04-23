//! Tenant-scoped SeaORM queries and commands for shifts, holidays, and attendance.

use chrono::{NaiveDate, Utc};
use kabipay_common::{KabiPayError, KabiPayResult};
use rust_decimal::Decimal;
use std::str::FromStr;
use kabipay_db_entities::tenant::d0010_time_shift_roster::{
    attendance, holiday, holiday_calendar, shift, timesheet_entry,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};
use std::collections::HashMap;
use uuid::Uuid;

pub async fn list_shifts(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<shift::Model>> {
    let limit = limit.clamp(1, 200);
    shift::Entity::find()
        .filter(shift::Column::TenantId.eq(tenant_id))
        .order_by_asc(shift::Column::Name)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_attendance(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<attendance::Model>> {
    let limit = limit.clamp(1, 500);
    attendance::Entity::find()
        .filter(attendance::Column::TenantId.eq(tenant_id))
        .order_by_desc(attendance::Column::WorkDate)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

/// Public holidays on or after `from`, ordered by date (tenant-wide: all
/// holiday calendars in the schema).
pub async fn list_upcoming_holidays(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    from: NaiveDate,
    limit: u64,
) -> KabiPayResult<Vec<(holiday::Model, String)>> {
    let limit = limit.clamp(1, 100);
    let cals = holiday_calendar::Entity::find()
        .filter(holiday_calendar::Column::TenantId.eq(tenant_id))
        .all(db)
        .await?;
    if cals.is_empty() {
        return Ok(vec![]);
    }
    let names: HashMap<Uuid, String> = cals.iter().map(|c| (c.id, c.name.clone())).collect();
    let cal_ids: Vec<Uuid> = cals.iter().map(|c| c.id).collect();
    let rows = holiday::Entity::find()
        .filter(holiday::Column::CalendarId.is_in(cal_ids))
        .filter(holiday::Column::HolidayDate.gte(from))
        .order_by_asc(holiday::Column::HolidayDate)
        .limit(limit)
        .all(db)
        .await?;
    let out: Vec<(holiday::Model, String)> = rows
        .into_iter()
        .filter_map(|h| {
            names
                .get(&h.calendar_id)
                .cloned()
                .map(|n| (h, n))
        })
        .collect();
    Ok(out)
}

/// Minutes in a single completed in→out pair (same calendar work_date).
fn segment_minutes(t_in: chrono::NaiveTime, t_out: chrono::NaiveTime) -> i32 {
    use chrono::Timelike;
    let s_in = t_in.num_seconds_from_midnight() as i64;
    let s_out = t_out.num_seconds_from_midnight() as i64;
    let d = s_out - s_in;
    if d <= 0 {
        return 0;
    }
    (d / 60) as i32
}

/// All attendance rows (segments) for one employee on one work day, ordered oldest first.
pub async fn list_employee_attendance_on_date(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    work_date: NaiveDate,
) -> KabiPayResult<Vec<attendance::Model>> {
    attendance::Entity::find()
        .filter(attendance::Column::TenantId.eq(tenant_id))
        .filter(attendance::Column::EmployeeId.eq(employee_id))
        .filter(attendance::Column::WorkDate.eq(work_date))
        .order_by_asc(attendance::Column::CreatedAt)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

/// Aggregated stats for a day: sum of (check-out − check-in) for every completed
/// segment, plus the current open segment (checked in, not out) if any.
pub struct PunchDaySummary {
    pub work_date: NaiveDate,
    pub total_worked_minutes: i32,
    pub open_segment: Option<attendance::Model>,
    pub segments: Vec<attendance::Model>,
}

pub async fn punch_day_summary(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    work_date: NaiveDate,
) -> KabiPayResult<PunchDaySummary> {
    let segments = list_employee_attendance_on_date(db, tenant_id, employee_id, work_date).await?;
    let mut total = 0i32;
    for row in &segments {
        if let (Some(tin), Some(tout)) = (row.check_in_time, row.check_out_time) {
            total += segment_minutes(tin, tout);
        }
    }
    let open_segment = segments
        .iter()
        .filter(|r| r.check_in_time.is_some() && r.check_out_time.is_none())
        .max_by_key(|r| r.created_at)
        .cloned();
    Ok(PunchDaySummary {
        work_date,
        total_worked_minutes: total,
        open_segment,
        segments,
    })
}

/// **Multi-segment punch:** each pair (punch in → punch out) is a separate `attendance` row
/// for the same `work_date`. The next call after a completed segment starts a new segment
/// (new check-in row). `total` worked time for the day is the sum of all completed segments.
pub async fn punch_today(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
) -> KabiPayResult<attendance::Model> {
    let now_ts = Utc::now();
    let today = now_ts.date_naive();
    let now_t = now_ts.naive_utc().time();

    let open = attendance::Entity::find()
        .filter(attendance::Column::TenantId.eq(tenant_id))
        .filter(attendance::Column::EmployeeId.eq(employee_id))
        .filter(attendance::Column::WorkDate.eq(today))
        .filter(attendance::Column::CheckInTime.is_not_null())
        .filter(attendance::Column::CheckOutTime.is_null())
        .order_by_desc(attendance::Column::CreatedAt)
        .one(db)
        .await?;

    if let Some(row) = open {
        let id = row.id;
        let mut am: attendance::ActiveModel = row.into();
        am.check_out_time = Set(Some(now_t));
        am.status = Set(Some("COMPLETE".into()));
        am.updated_at = Set(now_ts);
        am.update(db).await?;
        return attendance::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or_else(|| KabiPayError::Internal("attendance row missing after update".into()));
    }

    let id = Uuid::new_v4();
    let am = attendance::ActiveModel {
        id: Set(id),
        tenant_id: Set(tenant_id),
        employee_id: Set(employee_id),
        shift_id: Set(None),
        work_date: Set(today),
        check_in_time: Set(Some(now_t)),
        check_out_time: Set(None),
        check_in_lat: Set(None),
        check_in_lng: Set(None),
        check_out_lat: Set(None),
        check_out_lng: Set(None),
        source: Set(Some("WEB".into())),
        status: Set(Some("OPEN".into())),
        regularization_status: Set(None),
        biometric_ref: Set(None),
        overtime_hours: Set(None),
        late_minutes: Set(None),
        early_exit_minutes: Set(None),
        created_at: Set(now_ts),
        updated_at: Set(now_ts),
    };
    am.insert(db).await?;
    attendance::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("attendance row missing after insert".into()))
}

pub async fn list_timesheet_entries(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<timesheet_entry::Model>> {
    let limit = limit.clamp(1, 200);
    timesheet_entry::Entity::find()
        .filter(timesheet_entry::Column::TenantId.eq(tenant_id))
        .filter(timesheet_entry::Column::EmployeeId.eq(employee_id))
        .filter(timesheet_entry::Column::IsDeleted.eq(false))
        .order_by_desc(timesheet_entry::Column::WorkDate)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn create_timesheet_entry(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    work_date: NaiveDate,
    hours_worked: Decimal,
    project_code: Option<String>,
    description: Option<String>,
) -> KabiPayResult<timesheet_entry::Model> {
    if hours_worked <= Decimal::ZERO {
        return Err(KabiPayError::Validation(
            "hoursWorked must be greater than zero".into(),
        ));
    }
    let id = Uuid::new_v4();
    let now = Utc::now();
    let am = timesheet_entry::ActiveModel {
        id: Set(id),
        tenant_id: Set(tenant_id),
        employee_id: Set(employee_id),
        work_date: Set(work_date),
        hours_worked: Set(hours_worked),
        project_code: Set(project_code),
        description: Set(description),
        status: Set("DRAFT".into()),
        is_deleted: Set(false),
        deleted_at: Set(None),
        deleted_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    am.insert(db).await?;
    timesheet_entry::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("inserted timesheet_entry not found".into()))
}

pub async fn delete_timesheet_entry(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    entry_id: Uuid,
) -> KabiPayResult<bool> {
    let row = timesheet_entry::Entity::find()
        .filter(timesheet_entry::Column::Id.eq(entry_id))
        .filter(timesheet_entry::Column::TenantId.eq(tenant_id))
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "timesheet_entry",
            id: entry_id.to_string(),
        })?;
    if row.employee_id != employee_id {
        return Err(KabiPayError::Forbidden(
            "timesheet entry belongs to another employee".into(),
        ));
    }
    if row.is_deleted {
        return Ok(false);
    }
    let mut am: timesheet_entry::ActiveModel = row.into();
    am.is_deleted = Set(true);
    am.deleted_at = Set(Some(Utc::now()));
    am.updated_at = Set(Utc::now());
    am.update(db).await?;
    Ok(true)
}

pub fn parse_hours(s: &str) -> KabiPayResult<Decimal> {
    Decimal::from_str(s.trim()).map_err(|_| {
        KabiPayError::Validation("invalid hoursWorked; use a decimal string".into())
    })
}
