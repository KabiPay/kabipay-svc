//! SeaORM-backed queries and commands for the leave domain. Every query applies the
//! `tenant_id` filter (defence in depth on top of schema isolation) and
//! the `is_deleted = false` soft-delete filter.

use chrono::{Datelike, NaiveDate, Utc};
use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0011_leave::{
    leave_balance, leave_request, leave_type,
};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use uuid::Uuid;

pub async fn list_types(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<leave_type::Model>> {
    let limit = limit.clamp(1, 200);
    leave_type::Entity::find()
        .filter(leave_type::Column::TenantId.eq(tenant_id))
        .filter(leave_type::Column::IsDeleted.eq(false))
        .order_by_asc(leave_type::Column::Code)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_requests(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<leave_request::Model>> {
    let limit = limit.clamp(1, 200);
    leave_request::Entity::find()
        .filter(leave_request::Column::TenantId.eq(tenant_id))
        .filter(leave_request::Column::IsDeleted.eq(false))
        .order_by_desc(leave_request::Column::AppliedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_balances_for_employee(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    year: Option<i32>,
    limit: u64,
) -> KabiPayResult<Vec<leave_balance::Model>> {
    let limit = limit.clamp(1, 200);
    let mut q = leave_balance::Entity::find()
        .filter(leave_balance::Column::TenantId.eq(tenant_id))
        .filter(leave_balance::Column::EmployeeId.eq(employee_id));
    if let Some(y) = year {
        q = q.filter(leave_balance::Column::Year.eq(y));
    }
    q.order_by_asc(leave_balance::Column::Year)
        .order_by_asc(leave_balance::Column::LeaveTypeId)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

/// Submit a leave request in one transaction: validate leave type, ensure a
/// balance row exists, check remaining `balance_days`, insert `leave_request`
/// with status `PENDING`, and increase `pending_days` on the balance.
pub async fn submit_leave_request(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    leave_type_id: Uuid,
    from_date: NaiveDate,
    to_date: NaiveDate,
    is_half_day: bool,
    half_day_session: Option<String>,
    reason: Option<String>,
) -> KabiPayResult<leave_request::Model> {
    if from_date > to_date {
        return Err(KabiPayError::Validation(
            "fromDate must be on or before toDate".into(),
        ));
    }

    let days = requested_days(from_date, to_date, is_half_day)?;

    let txn = db.begin().await?;

    let lt = leave_type::Entity::find_by_id(leave_type_id)
        .filter(leave_type::Column::TenantId.eq(tenant_id))
        .filter(leave_type::Column::IsDeleted.eq(false))
        .one(&txn)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "leave_type",
            id: leave_type_id.to_string(),
        })?;

    if is_half_day && !lt.half_day_allowed {
        return Err(KabiPayError::Validation(
            "this leave type does not allow half-day requests".into(),
        ));
    }

    let year = from_date.year();
    let bal = leave_balance::Entity::find()
        .filter(leave_balance::Column::TenantId.eq(tenant_id))
        .filter(leave_balance::Column::EmployeeId.eq(employee_id))
        .filter(leave_balance::Column::LeaveTypeId.eq(leave_type_id))
        .filter(leave_balance::Column::Year.eq(year))
        .one(&txn)
        .await?
        .ok_or_else(|| {
            KabiPayError::Validation(
                "no leave balance for this leave type and year — ask HR to provision balances"
                    .into(),
            )
        })?;

    if bal.balance_days < days {
        return Err(KabiPayError::Validation(
            "insufficient leave balance for this request".into(),
        ));
    }

    let req_id = Uuid::new_v4();
    let now = Utc::now();
    let am_req = leave_request::ActiveModel {
        id: Set(req_id),
        tenant_id: Set(tenant_id),
        employee_id: Set(employee_id),
        leave_type_id: Set(leave_type_id),
        from_date: Set(from_date),
        to_date: Set(to_date),
        days_requested: Set(days),
        is_half_day: Set(is_half_day),
        half_day_session: Set(half_day_session),
        status: Set("PENDING".into()),
        reason: Set(reason),
        rejection_reason: Set(None),
        approved_by: Set(None),
        workflow_instance_id: Set(None),
        applied_at: Set(now),
        is_deleted: Set(false),
        deleted_at: Set(None),
        deleted_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    am_req.insert(&txn).await?;

    let new_pending = bal.pending_days + days;
    let new_balance = bal.balance_days - days;
    let mut am_bal: leave_balance::ActiveModel = bal.into();
    am_bal.pending_days = Set(new_pending);
    am_bal.balance_days = Set(new_balance);
    am_bal.update(&txn).await?;

    let model = leave_request::Entity::find_by_id(req_id)
        .one(&txn)
        .await?
        .ok_or_else(|| KabiPayError::Internal("inserted leave_request not found".into()))?;

    txn.commit().await?;
    Ok(model)
}

fn requested_days(
    from_date: NaiveDate,
    to_date: NaiveDate,
    is_half_day: bool,
) -> KabiPayResult<Decimal> {
    if is_half_day {
        if from_date != to_date {
            return Err(KabiPayError::Validation(
                "half-day leave must have the same fromDate and toDate".into(),
            ));
        }
        return Ok(Decimal::new(5, 1));
    }
    let n = (to_date - from_date).num_days() + 1;
    if n < 1 {
        return Err(KabiPayError::Validation(
            "fromDate must be on or before toDate".into(),
        ));
    }
    Ok(Decimal::from(n))
}
