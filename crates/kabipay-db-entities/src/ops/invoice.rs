//! `kabipay_ops.invoice`.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveEntityModel, DeriveRelation, EnumIter};
use uuid::Uuid;

type DateTimeUtc = DateTime<Utc>;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(schema_name = "kabipay_ops", table_name = "invoice")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub billing_cycle_id: Uuid,
    pub invoice_number: String,
    pub subtotal: Decimal,
    pub discount_total: Decimal,
    pub tax_amount: Decimal,
    pub total_amount: Decimal,
    pub currency: String,
    pub status: String,
    pub due_date: Option<NaiveDate>,
    pub sent_at: Option<DateTimeUtc>,
    pub paid_at: Option<DateTimeUtc>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTimeUtc>,
    pub deleted_by: Option<Uuid>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
