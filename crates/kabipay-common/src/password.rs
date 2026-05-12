//! Password hashing with argon2id (tenant user login verification uses this format).

use argon2::password_hash::{
    rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString,
};
use argon2::Argon2;

use crate::error::{KabiPayError, KabiPayResult};

/// Hash a plaintext password. Pass the returned string directly into the DB.
pub fn hash(plaintext: &str) -> KabiPayResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(plaintext.as_bytes(), &salt)
        .map_err(|e| KabiPayError::Internal(format!("argon2 hash failed: {e}")))?
        .to_string();
    Ok(hash)
}

/// Verify a plaintext password against a stored PHC hash.
pub fn verify(plaintext: &str, stored_hash: &str) -> KabiPayResult<bool> {
    let parsed = PasswordHash::new(stored_hash)
        .map_err(|e| KabiPayError::Internal(format!("invalid hash format: {e}")))?;
    Ok(Argon2::default()
        .verify_password(plaintext.as_bytes(), &parsed)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let hashed = hash("hunter2").unwrap();
        assert!(verify("hunter2", &hashed).unwrap());
        assert!(!verify("wrong", &hashed).unwrap());
    }
}
