//! Auto-generated from `kabipay-database/changelog/migrations/0013_tax_statutory/tax_statutory.xml`.

pub mod tax_configuration_version {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "tax_configuration_version")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub fiscal_year: i32,
        pub regime: Option<String>,
        pub country_code: String,
        pub is_active: bool,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}

pub mod tax_slab {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "tax_slab")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub tax_config_version_id: Uuid,
        pub income_from: Decimal,
        pub income_to: Option<Decimal>,
        pub tax_rate: Option<Decimal>,
        pub surcharge_rate: Option<Decimal>,
        pub cess_rate: Option<Decimal>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}

pub mod tax_computation {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "tax_computation")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub employee_id: Uuid,
        pub tax_config_version_id: Uuid,
        pub fiscal_year: i32,
        pub tax_regime_chosen: Option<String>,
        pub gross_income: Option<Decimal>,
        pub total_deductions: Option<Decimal>,
        pub taxable_income: Option<Decimal>,
        pub final_tax: Option<Decimal>,
        pub tds_per_month: Option<Decimal>,
        pub computed_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}

pub mod statutory_filing {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "statutory_filing")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub statutory_body_id: Option<Uuid>,
        pub filing_type: String,
        pub month: Option<i32>,
        pub year: i32,
        pub amount: Option<Decimal>,
        pub status: String,
        pub reference_number: Option<String>,
        pub filed_on: Option<NaiveDate>,
        pub file_storage_id: Option<Uuid>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}

pub mod form_16 {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "form_16")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub employee_id: Uuid,
        pub payroll_cycle_id: Option<Uuid>,
        pub fiscal_year: i32,
        pub file_storage_id: Option<Uuid>,
        pub generated_at: DateTimeUtc,
        pub is_sent_to_employee: bool,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}

pub mod labour_law_register {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "labour_law_register")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub register_name: String,
        pub register_type: Option<String>,
        pub year: i32,
        pub file_storage_id: Option<Uuid>,
        pub generated_at: DateTimeUtc,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}
