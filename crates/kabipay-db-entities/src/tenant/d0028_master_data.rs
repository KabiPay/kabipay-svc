//! Auto-generated from `kabipay-database/changelog/migrations/0028_master_data/master_data.xml`.

pub mod master_data {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "master_data")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub category: String,
        pub data_key: String,
        pub value: String,
        pub description: Option<String>,
        pub display_order: Option<i32>,
        pub is_system: bool,
        pub is_active: bool,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}
