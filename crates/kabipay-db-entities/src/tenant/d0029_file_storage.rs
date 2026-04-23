//! Auto-generated from `kabipay-database/changelog/migrations/0029_file_storage/file_storage.xml`.

pub mod file_storage {
    use crate::tenant::prelude::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "file_storage")]
    pub struct Model {
        #[sea_orm(primary_key, auto_increment = false)]
        pub id: Uuid,
        pub tenant_id: Uuid,
        pub provider: String,
        pub bucket: Option<String>,
        pub storage_path: String,
        pub original_filename: Option<String>,
        pub mime_type: Option<String>,
        pub file_size_bytes: Option<i64>,
        pub is_public: bool,
        pub uploaded_by: Option<Uuid>,
        pub created_at: DateTimeUtc,
        pub updated_at: DateTimeUtc,
    }

    impl ActiveModelBehavior for ActiveModel {}

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}
}
