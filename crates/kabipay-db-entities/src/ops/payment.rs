//! `kabipay_ops.payment`.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveEntityModel, DeriveRelation, EnumIter};
use uuid::Uuid;

type DateTimeUtc = DateTime<Utc>;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(schema_name = "kabipay_ops", table_name = "payment")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub amount: Decimal,
    pub payment_method: Option<String>,
    pub status: String,
    pub paid_at: Option<DateTimeUtc>,
    pub gateway_ref: Option<String>,
    pub failure_reason: Option<String>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
