//! Environment-driven storage configuration.

use kabipay_common::{KabiPayError, KabiPayResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileStorageMode {
    Local,
    S3Compat,
    #[allow(dead_code)]
    AzureBlob,
}

impl FileStorageMode {
    pub fn from_env() -> Self {
        let raw = std::env::var("KABIPAY_FILE_STORAGE_MODE").unwrap_or_else(|_| "local".into());
        let s = raw.trim().to_ascii_lowercase();
        match s.as_str() {
            "local" | "disk" => FileStorageMode::Local,
            "s3_compat" | "s3" | "r2" | "minio" => FileStorageMode::S3Compat,
            "azure" | "azure_blob" | "azblob" => FileStorageMode::AzureBlob,
            _ => FileStorageMode::Local,
        }
    }
}

#[derive(Debug, Clone)]
pub struct S3CompatSettings {
    pub endpoint: String,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub path_style: bool,
    pub bucket_prefix: String,
    pub per_tenant_bucket: bool,
    pub default_bucket: Option<String>,
}

impl S3CompatSettings {
    pub fn from_env() -> KabiPayResult<Self> {
        let endpoint = std::env::var("KABIPAY_S3_ENDPOINT")
            .map_err(|_| KabiPayError::Validation("KABIPAY_S3_ENDPOINT is required for s3_compat".into()))?
            .trim()
            .to_string();
        if endpoint.is_empty() {
            return Err(KabiPayError::Validation("KABIPAY_S3_ENDPOINT is empty".into()));
        }
        if !endpoint.starts_with("https://") && !endpoint.starts_with("http://") {
            return Err(KabiPayError::Validation(
                "KABIPAY_S3_ENDPOINT must be a full https:// or http:// URL".into(),
            ));
        }
        let access_key_id = std::env::var("KABIPAY_S3_ACCESS_KEY_ID")
            .map_err(|_| KabiPayError::Validation("KABIPAY_S3_ACCESS_KEY_ID is required for s3_compat".into()))?
            .trim()
            .to_string();
        if access_key_id.is_empty() {
            return Err(KabiPayError::Validation("KABIPAY_S3_ACCESS_KEY_ID is empty".into()));
        }
        let secret_access_key = std::env::var("KABIPAY_S3_SECRET_ACCESS_KEY")
            .map_err(|_| {
                KabiPayError::Validation("KABIPAY_S3_SECRET_ACCESS_KEY is required for s3_compat".into())
            })?
            .trim()
            .to_string();
        if secret_access_key.is_empty() {
            return Err(KabiPayError::Validation("KABIPAY_S3_SECRET_ACCESS_KEY is empty".into()));
        }

        let region = std::env::var("KABIPAY_S3_REGION").unwrap_or_else(|_| "auto".into());
        let bucket_prefix =
            std::env::var("KABIPAY_S3_BUCKET_PREFIX").unwrap_or_else(|_| "kabipay".into());
        if bucket_prefix.is_empty() {
            return Err(KabiPayError::Validation("KABIPAY_S3_BUCKET_PREFIX is empty".into()));
        }
        let per_tenant_bucket = !matches!(
            std::env::var("KABIPAY_S3_PER_TENANT_BUCKET")
                .unwrap_or_else(|_| "1".into())
                .to_ascii_lowercase()
                .as_str(),
            "0" | "false" | "no"
        );

        let default_bucket = std::env::var("KABIPAY_S3_DEFAULT_BUCKET")
            .ok()
            .and_then(|s| {
                let t = s.trim();
                if t.is_empty() {
                    None
                } else {
                    Some(t.to_string())
                }
            });
        if !per_tenant_bucket && default_bucket.is_none() {
            return Err(KabiPayError::Validation(
                "KABIPAY_S3_DEFAULT_BUCKET is required when KABIPAY_S3_PER_TENANT_BUCKET=0".into(),
            ));
        }

        let path_style = match std::env::var("KABIPAY_S3_PATH_STYLE").ok() {
            None => endpoint.to_ascii_lowercase().contains("r2.cloudflarestorage.com"),
            Some(v) => {
                let v = v.to_ascii_lowercase();
                v == "1" || v == "true" || v == "yes" || v == "path"
            }
        };

        Ok(S3CompatSettings {
            endpoint,
            region,
            access_key_id,
            secret_access_key,
            path_style,
            bucket_prefix,
            per_tenant_bucket,
            default_bucket,
        })
    }
}
