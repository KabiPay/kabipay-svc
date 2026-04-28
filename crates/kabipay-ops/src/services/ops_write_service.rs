//! Operator writes for billing and operator users.

use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
use argon2::Argon2;
use chrono::{Datelike, NaiveDate, Utc};
use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::ops::{billing_cycle, invoice, operator_user, payment, tenant};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set,
};
use uuid::Uuid;

const INVOICE_STATUSES: &[&str] = &[
    "DRAFT", "PENDING", "SENT", "PAID", "OVERDUE", "CANCELLED", "DISPUTED",
];
const PAYMENT_STATUSES: &[&str] = &["PENDING", "PROCESSING", "SUCCEEDED", "FAILED", "REFUNDED"];

fn month_period_containing(d: NaiveDate) -> (NaiveDate, NaiveDate) {
    let start =
        NaiveDate::from_ymd_opt(d.year(), d.month(), 1).expect("valid month start");
    let next_month = if d.month() == 12 {
        NaiveDate::from_ymd_opt(d.year() + 1, 1, 1).expect("valid next year")
    } else {
        NaiveDate::from_ymd_opt(d.year(), d.month() + 1, 1).expect("valid next month")
    };
    let end = next_month.pred_opt().expect("last day of month");
    (start, end)
}

pub async fn ensure_monthly_cycle(
    db: &DatabaseConnection,
    tenant_id: Uuid,
) -> KabiPayResult<billing_cycle::Model> {
    tenant::Entity::find_by_id(tenant_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "tenant",
            id: tenant_id.to_string(),
        })?;

    let today = Utc::now().date_naive();
    let (period_start, period_end) = month_period_containing(today);

    if let Some(row) = billing_cycle::Entity::find()
        .filter(billing_cycle::Column::TenantId.eq(tenant_id))
        .filter(billing_cycle::Column::PeriodStart.eq(period_start))
        .filter(billing_cycle::Column::PeriodEnd.eq(period_end))
        .one(db)
        .await?
    {
        return Ok(row);
    }

    let now = Utc::now();
    Ok(billing_cycle::ActiveModel {
        id: Set(Uuid::new_v4()),
        tenant_id: Set(tenant_id),
        period_start: Set(period_start),
        period_end: Set(period_end),
        frequency: Set("MONTHLY".into()),
        status: Set("PENDING".into()),
        auto_generated_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?)
}

pub async fn create_operator_user(
    db: &DatabaseConnection,
    email: String,
    password: String,
    full_name: String,
    phone: Option<String>,
) -> KabiPayResult<operator_user::Model> {
    let email = email.trim().to_lowercase();
    if email.is_empty() || !email.contains('@') {
        return Err(KabiPayError::Validation("email is invalid".into()));
    }
    if password.len() < 8 {
        return Err(KabiPayError::Validation(
            "password must be at least 8 characters".into(),
        ));
    }
    let full_name = full_name.trim().to_string();
    if full_name.is_empty() {
        return Err(KabiPayError::Validation("full_name must not be empty".into()));
    }

    if operator_user::Entity::find()
        .filter(operator_user::Column::Email.eq(email.clone()))
        .one(db)
        .await?
        .is_some()
    {
        return Err(KabiPayError::Conflict(format!(
            "operator user with email {email} already exists"
        )));
    }

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| KabiPayError::Internal(format!("argon2 hash failed: {e}")))?
        .to_string();

    let now = Utc::now();
    Ok(operator_user::ActiveModel {
        id: Set(Uuid::new_v4()),
        email: Set(email),
        password_hash: Set(password_hash),
        full_name: Set(full_name),
        phone: Set(phone),
        is_active: Set(true),
        last_login_at: Set(None),
        is_deleted: Set(false),
        deleted_at: Set(None),
        deleted_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?)
}

pub async fn create_invoice(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    billing_cycle_id: Option<Uuid>,
    subtotal: String,
    discount_total: Option<String>,
    tax_amount: Option<String>,
    total_amount: String,
    currency: String,
    status: Option<String>,
    due_date: Option<NaiveDate>,
) -> KabiPayResult<invoice::Model> {
    let status = status.unwrap_or_else(|| "PENDING".into());
    if !INVOICE_STATUSES.contains(&status.as_str()) {
        return Err(KabiPayError::Validation(format!("invalid invoice status {status}")));
    }

    let subtotal: Decimal = subtotal.parse().map_err(|_| {
        KabiPayError::Validation("subtotal is not a valid decimal".into())
    })?;
    let discount_total: Decimal = discount_total
        .as_deref()
        .unwrap_or("0")
        .parse()
        .map_err(|_| KabiPayError::Validation("discount_total is not a valid decimal".into()))?;
    let tax_amount: Decimal = tax_amount
        .as_deref()
        .unwrap_or("0")
        .parse()
        .map_err(|_| KabiPayError::Validation("tax_amount is not a valid decimal".into()))?;
    let total_amount: Decimal = total_amount.parse().map_err(|_| {
        KabiPayError::Validation("total_amount is not a valid decimal".into())
    })?;

    let currency = currency.trim().to_uppercase();
    if currency.len() != 3 {
        return Err(KabiPayError::Validation(
            "currency must be a 3-letter ISO code".into(),
        ));
    }

    tenant::Entity::find_by_id(tenant_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "tenant",
            id: tenant_id.to_string(),
        })?;

    let cycle_id = match billing_cycle_id {
        Some(id) => {
            let row = billing_cycle::Entity::find_by_id(id)
                .one(db)
                .await?
                .ok_or_else(|| KabiPayError::NotFound {
                    entity: "billing_cycle",
                    id: id.to_string(),
                })?;
            if row.tenant_id != tenant_id {
                return Err(KabiPayError::Validation(
                    "billing_cycle does not belong to tenant".into(),
                ));
            }
            id
        }
        None => ensure_monthly_cycle(db, tenant_id).await?.id,
    };

    let now = Utc::now();
    let invoice_number = format!(
        "INV-{}-{}",
        now.format("%Y%m%d%H%M%S"),
        Uuid::new_v4().to_string().split('-').next().unwrap_or("x")
    );

    Ok(invoice::ActiveModel {
        id: Set(Uuid::new_v4()),
        tenant_id: Set(tenant_id),
        billing_cycle_id: Set(cycle_id),
        invoice_number: Set(invoice_number),
        subtotal: Set(subtotal),
        discount_total: Set(discount_total),
        tax_amount: Set(tax_amount),
        total_amount: Set(total_amount),
        currency: Set(currency),
        status: Set(status),
        due_date: Set(due_date),
        sent_at: Set(None),
        paid_at: Set(None),
        is_deleted: Set(false),
        deleted_at: Set(None),
        deleted_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?)
}

pub async fn record_payment(
    db: &DatabaseConnection,
    invoice_id: Uuid,
    amount: String,
    payment_method: Option<String>,
    gateway_ref: Option<String>,
    status: Option<String>,
) -> KabiPayResult<payment::Model> {
    let status = status.unwrap_or_else(|| "SUCCEEDED".into());
    if !PAYMENT_STATUSES.contains(&status.as_str()) {
        return Err(KabiPayError::Validation(format!("invalid payment status {status}")));
    }

    let amount: Decimal = amount
        .parse()
        .map_err(|_| KabiPayError::Validation("amount is not a valid decimal".into()))?;

    let inv = invoice::Entity::find_by_id(invoice_id)
        .one(db)
        .await?
        .ok_or_else(|| KabiPayError::NotFound {
            entity: "invoice",
            id: invoice_id.to_string(),
        })?;
    if inv.is_deleted {
        return Err(KabiPayError::Validation("invoice is deleted".into()));
    }

    let now = Utc::now();
    let paid_at = if status == "SUCCEEDED" {
        Some(now)
    } else {
        None
    };

    let pay = payment::ActiveModel {
        id: Set(Uuid::new_v4()),
        invoice_id: Set(invoice_id),
        amount: Set(amount),
        payment_method: Set(payment_method),
        status: Set(status.clone()),
        paid_at: Set(paid_at),
        gateway_ref: Set(gateway_ref),
        failure_reason: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    }
    .insert(db)
    .await?;

    if status == "SUCCEEDED" {
        let paid_sum: Decimal = payment::Entity::find()
            .filter(payment::Column::InvoiceId.eq(invoice_id))
            .filter(payment::Column::Status.eq("SUCCEEDED"))
            .all(db)
            .await?
            .into_iter()
            .map(|p| p.amount)
            .sum();

        if paid_sum >= inv.total_amount {
            let mut am: invoice::ActiveModel = inv.into();
            am.paid_at = Set(Some(now));
            am.status = Set("PAID".into());
            am.updated_at = Set(now);
            am.update(db).await?;
        }
    }

    Ok(pay)
}
