//! tax_proof_line — employee deduction proofs (declared vs actual) with approval.

pub mod tax_proof_line {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "tax_proof_line")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub employee_id: Uuid,
        pub tax_config_version_id: Uuid,
        pub fiscal_year: i32,
        pub section_code: String,
        pub declared_amount: Decimal,
        pub actual_amount: Decimal,
        pub file_storage_id: Option<Uuid>,
        pub status: String,
        pub rejection_reason: Option<String>,
        pub approved_by: Option<Uuid>,
        pub submitted_at: DateTimeUtc,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}
