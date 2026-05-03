//! Auto-generated from `kabipay-database/changelog/migrations/0033_travel_request/travel_request.xml`.

pub mod travel_request {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "travel_request")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub employee_id: Uuid,
        pub origin_location: Option<String>,
        pub destination_location: Option<String>,
        pub from_date: NaiveDate,
        pub to_date: NaiveDate,
        pub purpose: String,
        pub estimated_amount: Option<Decimal>,
        pub currency: String,
        pub status: String,
        pub rejection_reason: Option<String>,
        pub approved_by: Option<Uuid>,
        pub rejected_by: Option<Uuid>,
        /// When set, approvals follow **`TRAVEL_REQUEST`** workflow steps (**M32** style).
        pub workflow_instance_id: Option<Uuid>,
        pub submitted_at: DateTimeUtc,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}
