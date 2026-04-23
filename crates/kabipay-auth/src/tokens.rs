//! Opaque refresh tokens.
//!
//! Refresh tokens are 256-bit random strings handed to the client as
//! base64url. The database stores only a SHA-256 hex digest so a DB dump
//! cannot be replayed against the auth service.

use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};

/// Generate a new refresh token. Returns `(raw, hash)` — give `raw` to the
/// client, persist `hash`.
pub fn generate_refresh() -> (String, String) {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let raw = base64_url_no_pad(&bytes);
    let hash = hash_refresh(&raw);
    (raw, hash)
}

/// Hash a raw refresh token for comparison against `token_hash` columns.
pub fn hash_refresh(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    hex::encode(hasher.finalize())
}

fn base64_url_no_pad(bytes: &[u8]) -> String {
    // Minimal url-safe base64 w/o padding. Avoids pulling the `base64`
    // crate for this single call-site.
    const ALPH: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity((bytes.len() * 4 + 2) / 3);
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i];
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] } else { 0 };
        let n = (u32::from(b0) << 16) | (u32::from(b1) << 8) | u32::from(b2);
        out.push(ALPH[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPH[((n >> 12) & 0x3f) as usize] as char);
        if i + 1 < bytes.len() {
            out.push(ALPH[((n >> 6) & 0x3f) as usize] as char);
        }
        if i + 2 < bytes.len() {
            out.push(ALPH[(n & 0x3f) as usize] as char);
        }
        i += 3;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_tokens_are_unique_and_deterministic_when_hashed() {
        let (raw1, h1) = generate_refresh();
        let (raw2, h2) = generate_refresh();
        assert_ne!(raw1, raw2);
        assert_ne!(h1, h2);
        assert_eq!(hash_refresh(&raw1), h1);
        assert_eq!(hash_refresh(&raw2), h2);
        assert_eq!(h1.len(), 64); // sha256 hex
    }

    #[test]
    fn base64_encoding_is_url_safe() {
        let encoded = base64_url_no_pad(&[0xff, 0xff, 0xff]);
        assert!(!encoded.contains('+') && !encoded.contains('/') && !encoded.contains('='));
    }
}
