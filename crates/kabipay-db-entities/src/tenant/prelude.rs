//! Shared imports for generated tenant entities.
pub use sea_orm::entity::prelude::*;
pub use sea_orm::prelude::Json;
pub use sea_orm::{
    ActiveModelBehavior, DeriveEntityModel, DeriveRelation, EnumIter, RelationTrait,
};
pub use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
pub use rust_decimal::Decimal;
pub use uuid::Uuid;

pub type DateTimeUtc = DateTime<Utc>;
