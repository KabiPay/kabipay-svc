//! Auto-generated from `kabipay-database/changelog/migrations/0032_attendance_punch_policy/attendance_punch_policy.xml`.

pub mod attendance_punch_policy {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "attendance_punch_policy")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub is_enforced: bool,
        pub site_latitude: Option<Decimal>,
        pub site_longitude: Option<Decimal>,
        pub max_distance_meters: Option<i32>,
        pub ip_allowlist: Option<String>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}
