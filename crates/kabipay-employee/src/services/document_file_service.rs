//! Local-disk `file_storage` + `employee_document` writes (M5 / Gap F).

use std::path::PathBuf;

use chrono::Utc;
use kabipay_common::{KabiPayError, KabiPayResult};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, DatabaseConnection, EntityTrait, TransactionTrait,
};
use uuid::Uuid;

use super::file_token::{sign_download_token, FileDownloadClaims};
use crate::entities::d0008_document_system::employee_document;
use crate::entities::d0029_file_storage::file_storage;

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

/// Persist `bytes` to disk and insert `file_storage` + `employee_document` (status PENDING).
pub async fn upload_employee_document(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    employee_id: Uuid,
    document_type_id: Uuid,
    uploader_user_id: Option<Uuid>,
    original_filename: String,
    mime_type: Option<String>,
    bytes: Vec<u8>,
) -> KabiPayResult<employee_document::Model> {
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

    let file_id = Uuid::new_v4();
    let doc_id = Uuid::new_v4();
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

    let txn = db.begin().await?;

    let fs_am = file_storage::ActiveModel {
        id: Set(file_id),
        tenant_id: Set(tenant_id),
        provider: Set(PROVIDER_LOCAL.into()),
        bucket: Set(None),
        storage_path: Set(rel),
        original_filename: Set(Some(original_filename)),
        mime_type: Set(mime_type),
        file_size_bytes: Set(Some(bytes.len() as i64)),
        is_public: Set(false),
        uploaded_by: Set(uploader_user_id),
        created_at: Set(now),
        updated_at: Set(now),
    };
    fs_am.insert(&txn).await.map_err(KabiPayError::from)?;

    let am = employee_document::ActiveModel {
        id: Set(doc_id),
        tenant_id: Set(tenant_id),
        employee_id: Set(employee_id),
        document_type_id: Set(document_type_id),
        file_storage_id: Set(Some(file_id)),
        status: Set("PENDING".into()),
        expiry_date: Set(None),
        workflow_instance_id: Set(None),
        uploaded_at: Set(now),
        verified_by: Set(None),
        verified_at: Set(None),
        is_deleted: Set(false),
        deleted_at: Set(None),
        deleted_by: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };
    am.insert(&txn).await.map_err(KabiPayError::from)?;
    txn.commit().await.map_err(KabiPayError::from)?;

    employee_document::Entity::find_by_id(doc_id)
        .one(db)
        .await
        .map_err(KabiPayError::from)?
        .ok_or_else(|| KabiPayError::Internal("inserted employee_document missing".into()))
}

/// Build signed claims for a short-TTL GET URL (no DB required on download).
pub fn download_claims(
    tenant_id: Uuid,
    file_storage_id: Uuid,
    mime_type: Option<String>,
    ttl_seconds: i64,
) -> FileDownloadClaims {
    FileDownloadClaims {
        tenant_id,
        file_storage_id,
        exp: Utc::now().timestamp() + ttl_seconds,
        mime_type,
    }
}

/// Build a time-limited HMAC download URL (HTTP GET) for a stored file.
pub fn public_download_url(claims: &FileDownloadClaims) -> String {
    let base = std::env::var("KABIPAY_EMPLOYEE_PUBLIC_BASE")
        .unwrap_or_else(|_| "http://127.0.0.1:4013".to_string());
    let token = sign_download_token(claims);
    let encoded = urlencoding::encode(&token);
    format!("{base}/files/employee-document?token={encoded}")
}
