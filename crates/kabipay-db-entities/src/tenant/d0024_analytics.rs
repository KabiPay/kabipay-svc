//! Auto-generated from `kabipay-database/changelog/migrations/0024_analytics/analytics.xml`.

pub mod report_definition {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "report_definition")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub name: String,
        pub entity_type: Option<String>,
        pub filters_json: Option<Json>,
        pub columns_json: Option<Json>,
        pub groupby_json: Option<Json>,
        pub chart_type: Option<String>,
        pub is_public: bool,
        pub created_by: Option<Uuid>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}

pub mod report_schedule {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "report_schedule")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub report_definition_id: Uuid,
        pub frequency: String,
        pub recipients_json: Option<Json>,
        pub delivery_format: Option<String>,
        pub last_sent_at: Option<DateTimeUtc>,
        pub next_run_at: Option<DateTimeUtc>,
        pub is_active: bool,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}

pub mod dashboard {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "dashboard")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub name: String,
        pub description: Option<String>,
        pub is_default: bool,
        pub created_by: Option<Uuid>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}

pub mod dashboard_widget {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "dashboard_widget")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub dashboard_id: Uuid,
        pub report_definition_id: Option<Uuid>,
        pub widget_type: Option<String>,
        pub title: Option<String>,
        pub grid_col: Option<i32>,
        pub grid_row: Option<i32>,
        pub col_span: Option<i32>,
        pub row_span: Option<i32>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}

pub mod workforce_snapshot {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "workforce_snapshot")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub snapshot_date: NaiveDate,
        pub total_headcount: Option<i32>,
        pub active_employees: Option<i32>,
        pub new_joiners: Option<i32>,
        pub separations: Option<i32>,
        pub open_positions: Option<i32>,
        pub average_tenure_months: Option<Decimal>,
        pub attrition_rate: Option<Decimal>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}
