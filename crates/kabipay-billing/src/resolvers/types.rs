//! GraphQL DTOs for kabipay-billing (ops plane).

use async_graphql::{SimpleObject, ID};
use chrono::{DateTime, NaiveDate, Utc};
use kabipay_db_entities::ops::{invoice, payment};

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Invoice")]
pub struct InvoiceDto {
    pub id: ID,
    pub tenant_id: ID,
    pub billing_cycle_id: ID,
    pub invoice_number: String,
    pub subtotal: String,
    pub discount_total: String,
    pub tax_amount: String,
    pub total_amount: String,
    pub currency: String,
    pub status: String,
    pub due_date: Option<NaiveDate>,
    pub sent_at: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<invoice::Model> for InvoiceDto {
    fn from(m: invoice::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            tenant_id: ID(m.tenant_id.to_string()),
            billing_cycle_id: ID(m.billing_cycle_id.to_string()),
            invoice_number: m.invoice_number,
            subtotal: m.subtotal.to_string(),
            discount_total: m.discount_total.to_string(),
            tax_amount: m.tax_amount.to_string(),
            total_amount: m.total_amount.to_string(),
            currency: m.currency,
            status: m.status,
            due_date: m.due_date,
            sent_at: m.sent_at,
            paid_at: m.paid_at,
            created_at: m.created_at,
        }
    }
}

#[derive(SimpleObject, Clone, Debug)]
#[graphql(name = "Payment")]
pub struct PaymentDto {
    pub id: ID,
    pub invoice_id: ID,
    pub amount: String,
    pub payment_method: Option<String>,
    pub status: String,
    pub paid_at: Option<DateTime<Utc>>,
    pub gateway_ref: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<payment::Model> for PaymentDto {
    fn from(m: payment::Model) -> Self {
        Self {
            id: ID(m.id.to_string()),
            invoice_id: ID(m.invoice_id.to_string()),
            amount: m.amount.to_string(),
            payment_method: m.payment_method,
            status: m.status,
            paid_at: m.paid_at,
            gateway_ref: m.gateway_ref,
            failure_reason: m.failure_reason,
            created_at: m.created_at,
        }
    }
}
