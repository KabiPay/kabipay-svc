//! `kabipay_ops.operator_user_role` junction (user ↔ role).

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelBehavior, DeriveEntityModel, DeriveRelation, EnumIter};
use uuid::Uuid;

type DateTimeUtc = DateTime<Utc>;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(schema_name = "kabipay_ops", table_name = "operator_user_role")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub operator_user_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub operator_role_id: Uuid,
    pub assigned_at: DateTimeUtc,
}

impl ActiveModelBehavior for ActiveModel {}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}
