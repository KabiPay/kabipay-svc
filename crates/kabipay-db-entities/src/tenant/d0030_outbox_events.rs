//! Auto-generated from `kabipay-database/changelog/migrations/0030_outbox_events/outbox_events.xml`.

pub mod outbox_event {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "outbox_event")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub aggregate_type: String,
        pub aggregate_id: Uuid,
        pub event_type: String,
        pub payload: Json,
        pub status: String,
        pub retry_count: i32,
        pub last_error: Option<String>,
        pub created_at: DateTimeUtc,
        pub processed_at: Option<DateTimeUtc>,
        /// When the worker moved this row to PROCESSING (for stale reclaim).
        pub claimed_at: Option<DateTimeUtc>,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}
