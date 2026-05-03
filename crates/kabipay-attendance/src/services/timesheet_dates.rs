//! Pure date helpers for Monday-start weeks (timesheet + locking).

use chrono::{Datelike, Duration, NaiveDate, Utc};

use super::hrms_master_service::TimesheetLockPolicy;

pub fn week_monday_sunday(anchor: NaiveDate) -> (NaiveDate, NaiveDate) {
    let off = anchor.weekday().num_days_from_monday();
    let mon = anchor - Duration::days(off as i64);
    let sun = mon + Duration::days(6);
    (mon, sun)
}

/// Earliest Monday start date employees may still **create/edit drafts** for, inclusive.
pub fn earliest_editable_week_start(policy: &TimesheetLockPolicy) -> NaiveDate {
    let today = Utc::now().date_naive();
    let (cur_mon, _) = week_monday_sunday(today);
    let span = policy.editable_week_span.max(1);
    cur_mon - Duration::days(7 * (span - 1))
}
