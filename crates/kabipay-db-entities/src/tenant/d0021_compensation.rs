//! Auto-generated from `kabipay-database/changelog/migrations/0021_compensation/compensation.xml`.

pub mod salary_band {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "salary_band")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub designation_id: Uuid,
        pub grade: Option<i32>,
        pub min_salary: Option<Decimal>,
        pub mid_salary: Option<Decimal>,
        pub max_salary: Option<Decimal>,
        pub currency: Option<String>,
        pub effective_year: Option<i32>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}

pub mod compensation_review_cycle {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "compensation_review_cycle")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub name: String,
        pub year: i32,
        pub start_date: NaiveDate,
        pub end_date: NaiveDate,
        pub status: String,
        pub budget_percentage: Option<Decimal>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}

pub mod compensation_review_item {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "compensation_review_item")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub review_cycle_id: Uuid,
        pub employee_id: Uuid,
        pub current_ctc: Option<Decimal>,
        pub recommended_ctc: Option<Decimal>,
        pub increment_percentage: Option<Decimal>,
        pub increment_type: Option<String>,
        pub status: String,
        pub approved_by: Option<Uuid>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}

pub mod bonus_plan {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "bonus_plan")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub name: String,
        pub calculation_basis: Option<String>,
        pub target_percentage: Option<Decimal>,
        pub review_cycle_id: Uuid,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}

pub mod bonus_payout {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "bonus_payout")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub bonus_plan_id: Uuid,
        pub employee_id: Uuid,
        pub target_amount: Option<Decimal>,
        pub actual_amount: Option<Decimal>,
        pub performance_multiplier: Option<Decimal>,
        pub status: String,
        pub approved_by: Option<Uuid>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}

pub mod equity_grant {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "equity_grant")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub employee_id: Uuid,
        pub grant_type: Option<String>,
        pub units_granted: Option<i32>,
        pub strike_price: Option<Decimal>,
        pub grant_date: Option<NaiveDate>,
        pub vesting_start_date: Option<NaiveDate>,
        pub vesting_schedule: Option<String>,
        pub status: String,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}
