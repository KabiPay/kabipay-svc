//! `kabipay_ops.tenant_subscription` — per-tenant module activations.

use chrono::{DateTime, NaiveDate, Utc};
use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveEntityModel, DeriveRelation, EnumIter};
use uuid::Uuid;

type DateTimeUtc = DateTime<Utc>;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(schema_name = "kabipay_ops", table_name = "tenant_subscription")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub tenant_id: Uuid,
    pub module_id: Uuid,
    pub status: String,
    pub activated_at: Option<NaiveDate>,
    pub expires_at: Option<NaiveDate>,
    pub contracted_seats: i32,
    pub current_seat_usage: i32,
    pub overage_policy: String,
    pub approved_by: Option<Uuid>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTimeUtc>,
    pub deleted_by: Option<Uuid>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
