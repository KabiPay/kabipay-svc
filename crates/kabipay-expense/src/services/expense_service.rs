//! Tenant-scoped SeaORM queries and commands for expenses.

use chrono::{NaiveDate, Utc};
use kabipay_common::client_data_scope::EmployeeScopeFilter;
use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0007_employee_core::employee;
use kabipay_db_entities::tenant::d0015_expense::{expense, expense_category};
use kabipay_db_entities::tenant::d0025_workflow::{
    workflow, workflow_action, workflow_instance, workflow_step,
};
use kabipay_db_entities::tenant::d0033_travel_request::travel_request;
use kabipay_db_entities::tenant::d0027_communication_audit::notification;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::str::FromStr;
use uuid::Uuid;

pub async fn list_categories(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
) -> KabiPayResult<Vec<expense_category::Model>> {
    let limit = limit.clamp(1, 200);
    expense_category::Entity::find()
        .filter(expense_category::Column::TenantId.eq(tenant_id))
        .filter(expense_category::Column::IsDeleted.eq(false))
        .order_by_asc(expense_category::Column::Code)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

pub async fn list_expenses(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    limit: u64,
    scope_filter: &EmployeeScopeFilter,
) -> KabiPayResult<Vec<expense::Model>> {
    let limit = limit.clamp(1, 200);
    match scope_filter {
        EmployeeScopeFilter::Empty => return Ok(vec![]),
        EmployeeScopeFilter::EmployeeIds(ids) if ids.is_empty() => return Ok(vec![]),
        _ => {}
    }
    let mut q = expense::Entity::find()
        .filter(expense::Column::TenantId.eq(tenant_id))
        .filter(expense::Column::IsDeleted.eq(false));
    if let EmployeeScopeFilter::EmployeeIds(ids) = scope_filter {
        q = q.filter(expense::Column::EmployeeId.is_in(ids.clone()));
    }
    q.order_by_desc(expense::Column::SubmittedAt)
        .limit(limit)
        .all(db)
        .await
        .map_err(KabiPayError::from)
}

/// Matches `workflow.entity_type` / `workflow_instance.entity_type` for expense (M32).
pub const WF_ENTITY_EXPENSE: &str = "EXPENSE";

const WF_STATUS_IN_PROGRESS: &str = "IN_PROGRESS";
const WF_STATUS_COMPLETED: &str = "COMPLETED";
const WF_STATUS_CANCELLED: &str = "CANCELLED";
const WF_ACTION_APPROVE: &str = "APPROVE";
const WF_ACTION_REJECT: &str = "REJECT";

/// Submit a new expense claim in `PENDING` status; validates category belongs to the tenant.
/// Optional `travel_request_id` links the claim to that employee’s trip.
///
/// When the tenant defines an active **`EXPENSE`** workflow with ≥1 step, inserts
/// **`workflow_instance`** (**M32**) and sets **`expense.workflow_instance_id`**.
pub async fn submit_expense(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    expense_category_id: Uuid,
    amount: Decimal,
    currency: &str,
    expense_date: NaiveDate,
    title: &str,
    travel_request_id: Option<Uuid>,
) -> KabiPayResult<expense::Model> {
    if amount <= Decimal::ZERO {
        return Err(KabiPayError::Validation(
            "amount must be greater than zero".into(),
        ));
    }

    let txn = db.begin().await?;

    let _cat = expense_category::Entity::find()
        .filter(expense_category::Column::Id.eq(expense_category_id))
        .filter(expense_category::Column::TenantId.eq(tenant_id))
        .filter(expense_category::Column::IsDeleted.eq(false))
        .one(&txn)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "expense_category",
            id: expense_category_id.to_string(),
        })?;

    if let Some(tid) = travel_request_id {
        let t = travel_request::Entity::find()
            .filter(travel_request::Column::Id.eq(tid))
            .filter(travel_request::Column::TenantId.eq(tenant_id))
            .one(&txn)
            .await?
            .ok_or_else(|| KabiPayError::NotFound {
                entity: "travel_request",
                id: tid.to_string(),
            })?;
        if t.employee_id != employee_id {
            return Err(KabiPayError::Validation(
                "travel request must belong to the submitting employee".into(),
            ));
        }
    }

    let id = Uuid::new_v4();
    let now = Utc::now();
    let am = expense::ActiveModel {
        id: Set(id),
        tenant_id: Set(tenant_id),
        employee_id: Set(employee_id),
        expense_category_id: Set(expense_category_id),
        amount: Set(amount),
        currency: Set(currency.to_string()),
        expense_date: Set(expense_date),
        title: Set(title.to_string()),
        status: Set("PENDING".into()),
        travel_request_id: Set(travel_request_id),
        rejection_reason: Set(None),
        approved_by: Set(None),
        workflow_instance_id: Set(None),
        submitted_at: Set(now),
        is_deleted: Set(false),
        deleted_at: Set(None),
        deleted_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    am.insert(&txn).await?;

    try_attach_expense_workflow(&txn, tenant_id, id, now).await?;

    txn.commit().await?;

    expense::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("inserted expense not found".into()))
}

async fn load_expense_workflow_first_step(
    db: &impl ConnectionTrait,
    tenant_id: Uuid,
) -> KabiPayResult<Option<(workflow::Model, Uuid)>> {
    let wf = workflow::Entity::find()
        .filter(workflow::Column::TenantId.eq(tenant_id))
        .filter(workflow::Column::IsActive.eq(true))
        .filter(workflow::Column::EntityType.eq(WF_ENTITY_EXPENSE))
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

async fn try_attach_expense_workflow(
    txn: &impl ConnectionTrait,
    tenant_id: Uuid,
    expense_id: Uuid,
    now: chrono::DateTime<Utc>,
) -> KabiPayResult<()> {
    let Some((wf, first_step_id)) = load_expense_workflow_first_step(txn, tenant_id).await? else {
        return Ok(());
    };
    let inst_id = Uuid::new_v4();
    let inst = workflow_instance::ActiveModel {
        id: Set(inst_id),
        tenant_id: Set(tenant_id),
        workflow_id: Set(wf.id),
        entity_type: Set(WF_ENTITY_EXPENSE.into()),
        entity_id: Set(expense_id),
        status: Set(WF_STATUS_IN_PROGRESS.into()),
        current_step_id: Set(Some(first_step_id)),
        created_at: Set(now),
        completed_at: Set(None),
        updated_at: Set(now),
    };
    inst.insert(txn).await.map_err(KabiPayError::from)?;

    let mut am_req: expense::ActiveModel = expense::Entity::find_by_id(expense_id)
        .one(txn)
        .await
        .map_err(KabiPayError::from)?
        .ok_or_else(|| KabiPayError::Internal("expense missing after insert".into()))?
        .into();
    am_req.workflow_instance_id = Set(Some(inst_id));
    am_req.update(txn).await.map_err(KabiPayError::from)?;
    Ok(())
}

/// Parse a decimal from string (GraphQL) into `Decimal`.
pub fn parse_amount(s: &str) -> KabiPayResult<Decimal> {
    Decimal::from_str(s.trim())
        .map_err(|_| KabiPayError::Validation("invalid amount; must be a decimal string".into()))
}

const STATUS_PENDING: &str = "PENDING";
const STATUS_APPROVED: &str = "APPROVED";
const STATUS_REJECTED: &str = "REJECTED";

/// Mark expense **APPROVED** + notify (**M32:** only after final workflow step, or legacy no-workflow approve).
async fn finalize_expense_approval(
    txn: &impl ConnectionTrait,
    tenant_id: Uuid,
    expense_id: Uuid,
    approver_user_id: Uuid,
    now: chrono::DateTime<Utc>,
) -> KabiPayResult<expense::Model> {
    let m = expense::Entity::find()
        .filter(expense::Column::Id.eq(expense_id))
        .filter(expense::Column::TenantId.eq(tenant_id))
        .one(txn)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "expense",
            id: expense_id.to_string(),
        })?;
    let mut am: expense::ActiveModel = m.into();
    am.status = Set(STATUS_APPROVED.into());
    am.rejection_reason = Set(None);
    am.approved_by = Set(Some(approver_user_id));
    am.updated_at = Set(now);
    am.update(txn).await?;
    expense::Entity::find()
        .filter(expense::Column::Id.eq(expense_id))
        .filter(expense::Column::TenantId.eq(tenant_id))
        .one(txn)
        .await?
        .ok_or_else(|| KabiPayError::Internal("updated expense not found".into()))
}

/// Approve routing: **`workflow_instance`** multi-step (**M32**) — intermediate steps advance the
/// instance without setting **APPROVED**; legacy **PENDING** with no workflow is single-step approve.
pub async fn approve_expense(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    expense_id: Uuid,
    approver_user_id: Uuid,
) -> KabiPayResult<expense::Model> {
    let txn = db.begin().await?;
    let model = load_pending_expense_conn(&txn, tenant_id, expense_id).await?;
    let now = Utc::now();

    if let Some(inst_id) = model.workflow_instance_id {
        let inst = workflow_instance::Entity::find_by_id(inst_id)
            .filter(workflow_instance::Column::TenantId.eq(tenant_id))
            .one(&txn)
            .await?
            .ok_or_else(|| KabiPayError::Validation("expense workflow_instance not found".into()))?;
        if inst.status != WF_STATUS_IN_PROGRESS {
            return Err(KabiPayError::Validation(
                "workflow instance is not in progress — cannot approve this expense".into(),
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
            return expense::Entity::find_by_id(expense_id)
                .one(db)
                .await?
                .ok_or_else(|| KabiPayError::Internal("expense missing after workflow step".into()));
        }

        let mut am_inst: workflow_instance::ActiveModel = inst.into();
        am_inst.status = Set(WF_STATUS_COMPLETED.into());
        am_inst.current_step_id = Set(None);
        am_inst.completed_at = Set(Some(now));
        am_inst.updated_at = Set(now);
        am_inst.update(&txn).await?;

        finalize_expense_approval(&txn, tenant_id, expense_id, approver_user_id, now).await?;
    } else {
        finalize_expense_approval(&txn, tenant_id, expense_id, approver_user_id, now).await?;
    }

    txn.commit().await?;

    let out = expense::Entity::find_by_id(expense_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::Internal("expense missing after approve".into()))?;

    expense_notify_employee(
        db,
        tenant_id,
        out.employee_id,
        "Expense approved",
        &format!("Your expense claim \"{}\" was approved.", out.title),
    )
    .await;
    Ok(out)
}

pub async fn reject_expense(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    expense_id: Uuid,
    rejector_user_id: Uuid,
    rejection_reason: Option<String>,
) -> KabiPayResult<expense::Model> {
    let txn = db.begin().await?;
    let model = load_pending_expense_conn(&txn, tenant_id, expense_id).await?;

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

    let now = Utc::now();
    let mut am: expense::ActiveModel = model.into();
    am.status = Set(STATUS_REJECTED.into());
    am.rejection_reason = Set(rejection_reason.clone());
    am.approved_by = Set(None);
    am.updated_at = Set(now);
    am.update(&txn).await?;

    let out = expense::Entity::find_by_id(expense_id)
        .one(&txn)
        .await?
        .ok_or_else(|| KabiPayError::Internal("updated expense not found".into()))?;

    txn.commit().await?;

    let msg = format!(
        "Your expense claim \"{}\" was rejected.{}",
        out.title,
        match &out.rejection_reason {
            Some(s) if !s.is_empty() => format!(" Reason: {s}"),
            _ => String::new(),
        }
    );
    expense_notify_employee(db, tenant_id, out.employee_id, "Expense rejected", &msg).await;
    Ok(out)
}

async fn load_pending_expense_conn(
    db: &impl ConnectionTrait,
    tenant_id: Uuid,
    expense_id: Uuid,
) -> KabiPayResult<expense::Model> {
    let m = expense::Entity::find()
        .filter(expense::Column::Id.eq(expense_id))
        .filter(expense::Column::TenantId.eq(tenant_id))
        .filter(expense::Column::IsDeleted.eq(false))
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "expense",
            id: expense_id.to_string(),
        })?;
    if m.status != STATUS_PENDING {
        return Err(KabiPayError::Validation(
            "only PENDING expenses can be approved or rejected".into(),
        ));
    }
    Ok(m)
}

async fn expense_notify_employee(
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
        r#type: Set(Some("EXPENSE".into())),
        title: Set(Some(title.into())),
        message: Set(Some(message.into())),
        action_url: Set(None),
        is_read: Set(false),
        read_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    if let Err(e) = am.insert(db).await {
        tracing::warn!(error = %e, "insert notification (expense) failed");
    }
}
