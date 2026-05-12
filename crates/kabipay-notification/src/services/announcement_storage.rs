//! `file_storage` rows for announcement attachments (no `employee_document`).

use std::path::PathBuf;

use chrono::Utc;
use kabipay_common::{KabiPayError, KabiPayResult};
use kabipay_db_entities::tenant::d0029_file_storage::file_storage;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, DatabaseConnection};
use uuid::Uuid;

use super::object_store::{
    FileStorageMode, S3CompatSettings, ensure_tenant_bucket, s3_operator_for_bucket, s3_put,
    tenant_bucket_name, PROVIDER_S3_COMPAT,
};

const PROVIDER_LOCAL: &str = "LOCAL";
const MAX_BYTES: usize = 6 * 1024 * 1024;

pub fn local_file_root() -> PathBuf {
    let root =
        std::env::var("KABIPAY_LOCAL_FILE_ROOT").unwrap_or_else(|_| "data/tenant_files".into());
    PathBuf::from(root)
}

fn absolute_storage_path(tenant_id: Uuid, file_id: Uuid) -> PathBuf {
    let mut p = local_file_root();
    p.push(tenant_id.to_string());
    p.push(format!("{file_id}"));
    p
}

/// Persist bytes to disk or object storage; returns new `file_storage.id`.
pub async fn store_blob(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    uploaded_by: Option<Uuid>,
    original_filename: String,
    mime_type: Option<String>,
    bytes: Vec<u8>,
) -> KabiPayResult<Uuid> {
    if bytes.is_empty() {
        return Err(KabiPayError::Validation(
            "upload file content must not be empty".into(),
        ));
    }
    if bytes.len() > MAX_BYTES {
        return Err(KabiPayError::Validation(format!(
            "file exceeds max size of {} bytes",
            MAX_BYTES
        )));
    }

    let mode = FileStorageMode::from_env();
    match mode {
        FileStorageMode::Local => {
            upload_local(
                db,
                tenant_id,
                uploaded_by,
                original_filename,
                mime_type,
                bytes,
            )
            .await
        }
        FileStorageMode::S3Compat => {
            let cfg = S3CompatSettings::from_env()?;
            let file_id = Uuid::new_v4();
            let now = Utc::now();
            let (bucket, storage_path): (String, String) = if cfg.per_tenant_bucket {
                let b = tenant_bucket_name(tenant_id, &cfg.bucket_prefix);
                ensure_tenant_bucket(&cfg, &b).await?;
                (b, file_id.to_string())
            } else {
                let b = cfg
                    .default_bucket
                    .as_ref()
                    .expect("validated in S3CompatSettings::from_env")
                    .clone();
                ensure_tenant_bucket(&cfg, &b).await?;
                (b, format!("{}/{}", tenant_id, file_id))
            };
            let sz = bytes.len() as i64;
            let op = s3_operator_for_bucket(&cfg, &bucket)?;
            s3_put(
                &op,
                &storage_path,
                bytes,
                mime_type.as_deref().filter(|s| !s.is_empty()),
            )
            .await?;
            insert_fs_row(
                db,
                tenant_id,
                uploaded_by,
                original_filename,
                mime_type,
                file_id,
                now,
                Some(bucket),
                storage_path,
                sz,
            )
            .await
        }
        FileStorageMode::AzureBlob => Err(KabiPayError::Validation(
            "KABIPAY_FILE_STORAGE_MODE=azure is not implemented yet.".into(),
        )),
    }
}

async fn upload_local(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    uploaded_by: Option<Uuid>,
    original_filename: String,
    mime_type: Option<String>,
    bytes: Vec<u8>,
) -> KabiPayResult<Uuid> {
    let file_id = Uuid::new_v4();
    let now = Utc::now();
    let rel = format!("{}/{}", tenant_id, file_id);
    let path = absolute_storage_path(tenant_id, file_id);
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e: std::io::Error| KabiPayError::Internal(format!("create_dir_all: {e}")))?;
    }
    tokio::fs::write(&path, &bytes)
        .await
        .map_err(|e| KabiPayError::Internal(format!("write local file: {e}")))?;
    let sz = bytes.len() as i64;
    insert_fs_row(
        db,
        tenant_id,
        uploaded_by,
        original_filename,
        mime_type,
        file_id,
        now,
        None,
        rel,
        sz,
    )
    .await
}

async fn insert_fs_row(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    uploaded_by: Option<Uuid>,
    original_filename: String,
    mime_type: Option<String>,
    file_id: Uuid,
    now: chrono::DateTime<Utc>,
    bucket: Option<String>,
    storage_path: String,
    size: i64,
) -> KabiPayResult<Uuid> {
    let provider = if bucket.is_some() {
        PROVIDER_S3_COMPAT.into()
    } else {
        PROVIDER_LOCAL.into()
    };
    let am = file_storage::ActiveModel {
        id: Set(file_id),
        tenant_id: Set(tenant_id),
        provider: Set(provider),
        bucket: Set(bucket),
        storage_path: Set(storage_path),
        original_filename: Set(Some(original_filename)),
        mime_type: Set(mime_type),
        file_size_bytes: Set(Some(size)),
        is_public: Set(false),
        uploaded_by: Set(uploaded_by),
        created_at: Set(now),
        updated_at: Set(now),
    };
    am.insert(db).await.map_err(KabiPayError::from)?;
    Ok(file_id)
}
