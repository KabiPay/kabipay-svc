//! HMAC-signed, short-TTL tokens for unauthenticated HTTP GET file downloads
//! (same secret as JWT in dev; split in production if needed).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

use crate::jwt::jwt_secret_from_env;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Serialize, Deserialize)]
pub struct FileDownloadClaims {
    pub tenant_id: Uuid,
    pub file_storage_id: Uuid,
    pub exp: i64,
    #[serde(default)]
    pub mime_type: Option<String>,
}

/// Build `payload_b64.hmac_b64` wire token.
pub fn sign_download_token(claims: &FileDownloadClaims) -> String {
    let json = serde_json::to_string(claims).expect("serialize file claims");
    let mut mac =
        HmacSha256::new_from_slice(&jwt_secret_from_env()).expect("HMAC can take key of any size");
    mac.update(json.as_bytes());
    let tag = mac.finalize().into_bytes();
    let p = URL_SAFE_NO_PAD.encode(json);
    let s = URL_SAFE_NO_PAD.encode(tag);
    format!("{p}.{s}")
}

/// Verify signature and return claims if not expired.
pub fn verify_download_token(token: &str) -> Option<FileDownloadClaims> {
    let (p, s) = token.split_once('.')?;
    let json = String::from_utf8(URL_SAFE_NO_PAD.decode(p).ok()?).ok()?;
    let tag = URL_SAFE_NO_PAD.decode(s).ok()?;
    let mut mac = HmacSha256::new_from_slice(&jwt_secret_from_env()).ok()?;
    mac.update(json.as_bytes());
    mac.verify_slice(&tag).ok()?;
    let c: FileDownloadClaims = serde_json::from_str(&json).ok()?;
    if c.exp < chrono::Utc::now().timestamp() {
        return None;
    }
    Some(c)
}

/// Claims for a time-limited download (used by GraphQL resolvers before signing).
pub fn file_download_claims(
    tenant_id: Uuid,
    file_storage_id: Uuid,
    mime_type: Option<String>,
    ttl_seconds: i64,
) -> FileDownloadClaims {
    FileDownloadClaims {
        tenant_id,
        file_storage_id,
        exp: chrono::Utc::now().timestamp() + ttl_seconds,
        mime_type,
    }
}

/// Full URL for `GET /files/employee-document?token=…` on **kabipay-employee** (port from env).
pub fn public_employee_file_download_url(claims: &FileDownloadClaims) -> String {
    let base = std::env::var("KABIPAY_EMPLOYEE_PUBLIC_BASE")
        .unwrap_or_else(|_| "http://127.0.0.1:4013".to_string());
    let token = sign_download_token(claims);
    let encoded = urlencoding::encode(&token);
    format!("{base}/files/employee-document?token={encoded}")
}
