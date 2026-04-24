//! SeaORM-backed queries and commands for the leave domain. Every query applies the
//! `tenant_id` filter (defence in depth on top of schema isolation) and
//! the `is_deleted = false` soft-delete filter.

use chrono::{Datelike, NaiveDate, Utc};
use kabipay_common::context::{ClientViewerEmployee, ScopeType};
use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0007_employee_core::employee;
use kabipay_db_entities::tenant::d0011_leave::{leave_balance, leave_request, leave_type};
use kabipay_db_entities::tenant::d0025_workflow::{
    workflow, workflow_action, workflow_instance, workflow_step,
};
use kabipay_db_entities::tenant::d0027_communication_audit::notification;
use kabipay_db_entities::tenant::d0030_outbox_events::outbox_event;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
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

/// Leave requests the caller is allowed to see: `All` = tenant list; `Self` = own; `Team` = self +
/// direct reports; `Department` = same department; missing `viewer` for non-All → empty.
pub async fn list_requests(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
    scope: ScopeType,
    viewer: Option<ClientViewerEmployee>,
) -> KabiPayResult<Vec<leave_request::Model>> {
    let limit = limit.clamp(1, 200);
    let mut q = leave_request::Entity::find()
        .filter(leave_request::Column::TenantId.eq(tenant_id))
        .filter(leave_request::Column::IsDeleted.eq(false));

    match scope {
        ScopeType::All => {}
        ScopeType::Self_ => {
            let Some(v) = viewer else {
                return Ok(vec![]);
            };
            q = q.filter(leave_request::Column::EmployeeId.eq(v.employee_id));
        }
        ScopeType::Team => {
            let Some(v) = viewer else {
                return Ok(vec![]);
            };
            let ids = team_member_employee_ids(db, tenant_id, v.employee_id).await?;
            if ids.is_empty() {
                return Ok(vec![]);
            }
            q = q.filter(leave_request::Column::EmployeeId.is_in(ids));
        }
        ScopeType::Department => {
            let Some(v) = viewer else {
                return Ok(vec![]);
            };
            let ids = department_peer_employee_ids(db, tenant_id, v).await?;
            if ids.is_empty() {
                return Ok(vec![]);
            }
            q = q.filter(leave_request::Column::EmployeeId.is_in(ids));
        }
    }

    q.order_by_desc(leave_request::Column::AppliedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

/// Viewer plus everyone who reports to them.
async fn team_member_employee_ids(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    manager_employee_id: Uuid,
) -> KabiPayResult<Vec<Uuid>> {
    let reports = employee::Entity::find()
        .filter(employee::Column::TenantId.eq(tenant_id))
        .filter(employee::Column::IsDeleted.eq(false))
        .filter(employee::Column::ReportingManagerId.eq(manager_employee_id))
        .all(db)
        .await
        .map_err(KabiPayError::from)?;
    let mut ids: Vec<Uuid> = reports.into_iter().map(|e| e.id).collect();
    ids.push(manager_employee_id);
    Ok(ids)
}

/// Everyone in the same department, or only the caller when they have no department.
async fn department_peer_employee_ids(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    viewer: ClientViewerEmployee,
) -> KabiPayResult<Vec<Uuid>> {
    let Some(d) = viewer.department_id else {
        return Ok(vec![viewer.employee_id]);
    };
    let rows = employee::Entity::find()
        .filter(employee::Column::TenantId.eq(tenant_id))
        .filter(employee::Column::IsDeleted.eq(false))
        .filter(employee::Column::DepartmentId.eq(Some(d)))
        .all(db)
        .await
        .map_err(KabiPayError::from)?;
    Ok(rows.into_iter().map(|e| e.id).collect())
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

    try_attach_leave_workflow(&txn, tenant_id, req_id, now).await?;

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

const STATUS_PENDING: &str = "PENDING";
const STATUS_APPROVED: &str = "APPROVED";
const STATUS_REJECTED: &str = "REJECTED";

/// New outbox rows start here until a consumer marks them processed (Gap G — M6).
const OUTBOX_STATUS_PENDING: &str = "PENDING";

/// Matches `workflow.entity_type` / `workflow_instance.entity_type` for leave (seed + M8).
const WF_ENTITY_LEAVE: &str = "LEAVE_REQUEST";
const WF_STATUS_IN_PROGRESS: &str = "IN_PROGRESS";
const WF_STATUS_COMPLETED: &str = "COMPLETED";
const WF_STATUS_CANCELLED: &str = "CANCELLED";
const WF_ACTION_APPROVE: &str = "APPROVE";
const WF_ACTION_REJECT: &str = "REJECT";

async fn load_leave_workflow_first_step(
    db: &impl ConnectionTrait,
    tenant_id: Uuid,
) -> KabiPayResult<Option<(workflow::Model, Uuid)>> {
    let wf = workflow::Entity::find()
        .filter(workflow::Column::TenantId.eq(tenant_id))
        .filter(workflow::Column::IsActive.eq(true))
        .filter(workflow::Column::EntityType.eq(WF_ENTITY_LEAVE))
        .order_by_asc(workflow::Column::Name)
        .one(db)
        .await
        .map_err(KabiPayError::from)?;
    let Some(wf) = wf else {
        return Ok(None);
    };
    let step = workflow_step::Entity::find()
        .filter(workflow_step::Column::TenantId.eq(tenant_id))
        .filter(workflow_step::Column::WorkflowId.eq(wf.id))
        .order_by_asc(workflow_step::Column::SequenceOrder)
        .one(db)
        .await
        .map_err(KabiPayError::from)?;
    let Some(step) = step else {
        return Ok(None);
    };
    Ok(Some((wf, step.id)))
}

async fn try_attach_leave_workflow(
    txn: &impl ConnectionTrait,
    tenant_id: Uuid,
    leave_request_id: Uuid,
    now: chrono::DateTime<Utc>,
) -> KabiPayResult<()> {
    let Some((wf, first_step_id)) = load_leave_workflow_first_step(txn, tenant_id).await? else {
        return Ok(());
    };
    let inst_id = Uuid::new_v4();
    let inst = workflow_instance::ActiveModel {
        id: Set(inst_id),
        tenant_id: Set(tenant_id),
        workflow_id: Set(wf.id),
        entity_type: Set(WF_ENTITY_LEAVE.into()),
        entity_id: Set(leave_request_id),
        status: Set(WF_STATUS_IN_PROGRESS.into()),
        current_step_id: Set(Some(first_step_id)),
        created_at: Set(now),
        completed_at: Set(None),
        updated_at: Set(now),
    };
    inst.insert(txn).await.map_err(KabiPayError::from)?;

    let mut am_req: leave_request::ActiveModel =
        leave_request::Entity::find_by_id(leave_request_id)
            .one(txn)
            .await
            .map_err(KabiPayError::from)?
            .ok_or_else(|| KabiPayError::Internal("leave_request missing after insert".into()))?
            .into();
    am_req.workflow_instance_id = Set(Some(inst_id));
    am_req.update(txn).await.map_err(KabiPayError::from)?;
    Ok(())
}

async fn load_balance_for_request(
    txn: &impl ConnectionTrait,
    tenant_id: Uuid,
    model: &leave_request::Model,
) -> KabiPayResult<leave_balance::Model> {
    let year = model.from_date.year();
    leave_balance::Entity::find()
        .filter(leave_balance::Column::TenantId.eq(tenant_id))
        .filter(leave_balance::Column::EmployeeId.eq(model.employee_id))
        .filter(leave_balance::Column::LeaveTypeId.eq(model.leave_type_id))
        .filter(leave_balance::Column::Year.eq(year))
        .one(txn)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "leave_balance",
            id: format!("{}-{}", model.employee_id, year),
        })
}

/// Final approval: leave row APPROVED, balance pending→used, **`outbox_event`** (M6).
async fn finalize_leave_approval(
    txn: &impl ConnectionTrait,
    tenant_id: Uuid,
    model: &leave_request::Model,
    bal: &leave_balance::Model,
    approver_user_id: Uuid,
    now: chrono::DateTime<Utc>,
    request_id: Uuid,
) -> KabiPayResult<()> {
    let days = model.days_requested;
    let new_pending = bal.pending_days - days;
    if new_pending < Decimal::ZERO {
        return Err(KabiPayError::Validation(
            "leave balance pending mismatch — cannot approve".into(),
        ));
    }
    let new_used = bal.used_days + days;

    let mut am_req: leave_request::ActiveModel = model.clone().into();
    am_req.status = Set(STATUS_APPROVED.into());
    am_req.rejection_reason = Set(None);
    am_req.approved_by = Set(Some(approver_user_id));
    am_req.updated_at = Set(now);
    am_req.update(txn).await?;

    let mut am_bal: leave_balance::ActiveModel = bal.clone().into();
    am_bal.pending_days = Set(new_pending);
    am_bal.used_days = Set(new_used);
    am_bal.updated_at = Set(now);
    am_bal.update(txn).await?;

    let out = leave_request::Entity::find_by_id(request_id)
        .one(txn)
        .await?
        .ok_or_else(|| KabiPayError::Internal("updated leave_request not found".into()))?;

    let payload = serde_json::json!({
        "schema_version": 1,
        "leave_request_id": out.id,
        "employee_id": out.employee_id,
        "leave_type_id": out.leave_type_id,
        "approver_user_id": approver_user_id,
        "from_date": out.from_date.to_string(),
        "to_date": out.to_date.to_string(),
        "days_requested": out.days_requested.normalize().to_string(),
        "is_half_day": out.is_half_day,
        "status": out.status,
    });
    let ob = outbox_event::ActiveModel {
        id: Set(Uuid::new_v4()),
        tenant_id: Set(tenant_id),
        aggregate_type: Set("leave_request".into()),
        aggregate_id: Set(request_id),
        event_type: Set("leave_request.approved".into()),
        payload: Set(payload),
        status: Set(OUTBOX_STATUS_PENDING.into()),
        retry_count: Set(0),
        last_error: Set(None),
        created_at: Set(now),
        processed_at: Set(None),
    };
    ob.insert(txn).await?;
    Ok(())
}

/// Set request to APPROVED, `approved_by` = `approver_user_id` (user.id), and move
/// `pending_days` → `used_days` on the annual balance (submit already reserved balance).
///
/// When **`workflow_instance_id`** is set (**M8**), records **`workflow_action`**, advances
/// **`workflow_instance.current_step_id`** until the last step; only the **final** step
/// performs balance movement and emits **`outbox_event`** (same as M6).
pub async fn approve_leave_request(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    request_id: Uuid,
    approver_user_id: Uuid,
) -> KabiPayResult<leave_request::Model> {
    let txn = db.begin().await?;
    let model = load_pending_request_in_txn(&txn, tenant_id, request_id).await?;
    let now = Utc::now();

    if let Some(inst_id) = model.workflow_instance_id {
        let inst = workflow_instance::Entity::find_by_id(inst_id)
            .filter(workflow_instance::Column::TenantId.eq(tenant_id))
            .one(&txn)
            .await?
            .ok_or_else(|| {
                KabiPayError::Validation("leave_request workflow_instance not found".into())
            })?;
        if inst.status != WF_STATUS_IN_PROGRESS {
            return Err(KabiPayError::Validation(
                "workflow instance is not in progress — cannot approve this leave".into(),
            ));
        }
        let cur_step_id = inst.current_step_id.ok_or_else(|| {
            KabiPayError::Validation("workflow instance has no current step".into())
        })?;
        let cur_step = workflow_step::Entity::find_by_id(cur_step_id)
            .filter(workflow_step::Column::TenantId.eq(tenant_id))
            .one(&txn)
            .await?
            .ok_or_else(|| KabiPayError::Validation("workflow step not found".into()))?;

        let act = workflow_action::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(tenant_id),
            instance_id: Set(inst_id),
            workflow_step_id: Set(cur_step_id),
            performed_by: Set(Some(approver_user_id)),
            action: Set(WF_ACTION_APPROVE.into()),
            remarks: Set(None),
            acted_at: Set(now),
            created_at: Set(now),
            updated_at: Set(now),
        };
        act.insert(&txn).await?;

        let next_step = workflow_step::Entity::find()
            .filter(workflow_step::Column::TenantId.eq(tenant_id))
            .filter(workflow_step::Column::WorkflowId.eq(inst.workflow_id))
            .filter(workflow_step::Column::SequenceOrder.gt(cur_step.sequence_order))
            .order_by_asc(workflow_step::Column::SequenceOrder)
            .one(&txn)
            .await?;

        if let Some(next) = next_step {
            let mut am_inst: workflow_instance::ActiveModel = inst.into();
            am_inst.current_step_id = Set(Some(next.id));
            am_inst.updated_at = Set(now);
            am_inst.update(&txn).await?;
            txn.commit().await?;
            return leave_request::Entity::find_by_id(request_id)
                .one(db)
                .await?
                .ok_or_else(|| {
                    KabiPayError::Internal("leave_request missing after commit".into())
                });
        }

        let mut am_inst: workflow_instance::ActiveModel = inst.into();
        am_inst.status = Set(WF_STATUS_COMPLETED.into());
        am_inst.current_step_id = Set(None);
        am_inst.completed_at = Set(Some(now));
        am_inst.updated_at = Set(now);
        am_inst.update(&txn).await?;

        let bal = load_balance_for_request(&txn, tenant_id, &model).await?;
        finalize_leave_approval(
            &txn,
            tenant_id,
            &model,
            &bal,
            approver_user_id,
            now,
            request_id,
        )
        .await?;
    } else {
        let bal = load_balance_for_request(&txn, tenant_id, &model).await?;
        finalize_leave_approval(
            &txn,
            tenant_id,
            &model,
            &bal,
            approver_user_id,
            now,
            request_id,
        )
        .await?;
    }

    txn.commit().await?;
    let out = leave_request::Entity::find_by_id(request_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("leave_request missing after commit".into()))?;

    leave_notify_employee(
        db,
        tenant_id,
        out.employee_id,
        "Leave approved",
        "Your leave request was approved.",
    )
    .await;
    Ok(out)
}

/// Reject a PENDING request, release the balance hold, and optionally record a reason.
/// Cancels an in-progress **`workflow_instance`** when **`workflow_instance_id`** is set (**M8**).
pub async fn reject_leave_request(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    request_id: Uuid,
    rejector_user_id: Uuid,
    rejection_reason: Option<String>,
) -> KabiPayResult<leave_request::Model> {
    let txn = db.begin().await?;
    let model = load_pending_request_in_txn(&txn, tenant_id, request_id).await?;

    if let Some(inst_id) = model.workflow_instance_id {
        if let Some(inst) = workflow_instance::Entity::find_by_id(inst_id)
            .filter(workflow_instance::Column::TenantId.eq(tenant_id))
            .one(&txn)
            .await?
        {
            if inst.status == WF_STATUS_IN_PROGRESS {
                let now = Utc::now();
                if let Some(step_id) = inst.current_step_id {
                    let act = workflow_action::ActiveModel {
                        id: Set(Uuid::new_v4()),
                        tenant_id: Set(tenant_id),
                        instance_id: Set(inst_id),
                        workflow_step_id: Set(step_id),
                        performed_by: Set(Some(rejector_user_id)),
                        action: Set(WF_ACTION_REJECT.into()),
                        remarks: Set(rejection_reason.clone()),
                        acted_at: Set(now),
                        created_at: Set(now),
                        updated_at: Set(now),
                    };
                    act.insert(&txn).await?;
                }
                let mut am_inst: workflow_instance::ActiveModel = inst.into();
                am_inst.status = Set(WF_STATUS_CANCELLED.into());
                am_inst.completed_at = Set(Some(now));
                am_inst.updated_at = Set(now);
                am_inst.update(&txn).await?;
            }
        }
    }

    let year = model.from_date.year();
    let days = model.days_requested;
    let bal = leave_balance::Entity::find()
        .filter(leave_balance::Column::TenantId.eq(tenant_id))
        .filter(leave_balance::Column::EmployeeId.eq(model.employee_id))
        .filter(leave_balance::Column::LeaveTypeId.eq(model.leave_type_id))
        .filter(leave_balance::Column::Year.eq(year))
        .one(&txn)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "leave_balance",
            id: format!("{}-{}", model.employee_id, year),
        })?;

    let new_pending = bal.pending_days - days;
    if new_pending < Decimal::ZERO {
        return Err(KabiPayError::Validation(
            "leave balance pending mismatch — cannot reject".into(),
        ));
    }
    let new_balance = bal.balance_days + days;

    let now = Utc::now();
    let mut am_req: leave_request::ActiveModel = model.clone().into();
    am_req.status = Set(STATUS_REJECTED.into());
    am_req.rejection_reason = Set(rejection_reason);
    am_req.approved_by = Set(None);
    am_req.updated_at = Set(now);
    am_req.update(&txn).await?;

    let mut am_bal: leave_balance::ActiveModel = bal.into();
    am_bal.pending_days = Set(new_pending);
    am_bal.balance_days = Set(new_balance);
    am_bal.updated_at = Set(now);
    am_bal.update(&txn).await?;

    let out = leave_request::Entity::find_by_id(request_id)
        .one(&txn)
        .await?
        .ok_or_else(|| KabiPayError::Internal("updated leave_request not found".into()))?;
    txn.commit().await?;
    let msg = match &out.rejection_reason {
        Some(s) if !s.is_empty() => format!("Your leave was rejected. Reason: {s}"),
        _ => "Your leave request was rejected.".into(),
    };
    leave_notify_employee(db, tenant_id, out.employee_id, "Leave rejected", &msg).await;
    Ok(out)
}

async fn load_pending_request_in_txn(
    db: &impl ConnectionTrait,
    tenant_id: Uuid,
    request_id: Uuid,
) -> KabiPayResult<leave_request::Model> {
    let m = leave_request::Entity::find_by_id(request_id)
        .filter(leave_request::Column::TenantId.eq(tenant_id))
        .filter(leave_request::Column::IsDeleted.eq(false))
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "leave_request",
            id: request_id.to_string(),
        })?;
    if m.status != STATUS_PENDING {
        return Err(KabiPayError::Validation(
            "only PENDING leave requests can be approved or rejected".into(),
        ));
    }
    Ok(m)
}

/// Best-effort in-app row for the requester's linked `user` (if any).
async fn leave_notify_employee(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    title: &str,
    message: &str,
) {
    let user_id: Option<Uuid> = match employee::Entity::find_by_id(employee_id)
        .filter(employee::Column::TenantId.eq(tenant_id))
        .filter(employee::Column::IsDeleted.eq(false))
        .one(db)
        .await
    {
        Ok(Some(emp)) => emp.user_id,
        _ => None,
    };
    let Some(user_id) = user_id else {
        return;
    };
    let now = Utc::now();
    let am = notification::ActiveModel {
        id: Set(Uuid::new_v4()),
        tenant_id: Set(tenant_id),
        user_id: Set(user_id),
        r#type: Set(Some("LEAVE".into())),
        title: Set(Some(title.into())),
        message: Set(Some(message.into())),
        action_url: Set(None),
        is_read: Set(false),
        read_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    if let Err(e) = am.insert(db).await {
        tracing::warn!(error = %e, "insert notification (leave) failed");
    }
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
