//! `kabipay_ops.tenant`.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveEntityModel, DeriveRelation, EnumIter};
use uuid::Uuid;

type DateTimeUtc = DateTime<Utc>;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(schema_name = "kabipay_ops", table_name = "tenant")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub name: String,
    pub status: String,
    pub plan: Option<String>,
    pub country: Option<String>,
    pub timezone: Option<String>,
    pub currency: Option<String>,
    pub gstin: Option<String>,
    pub pan: Option<String>,
    pub registered_address: Option<String>,
    pub logo_url: Option<String>,
    pub primary_color: Option<String>,
    pub subdomain: Option<String>,
    pub account_manager_id: Option<Uuid>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTimeUtc>,
    pub deleted_by: Option<Uuid>,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
