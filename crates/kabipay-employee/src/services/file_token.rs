//! Short-TTL HMAC token for unauthenticated `GET` download of a tenant file
//! (same secret as JWT for dev simplicity; split in production if needed).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use hmac::{Hmac, Mac};
use kabipay_common::jwt::jwt_secret_from_env;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

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
