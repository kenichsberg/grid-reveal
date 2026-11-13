//! Password hashing.
//!
//! argon2id at the crate's default parameters, which track the OWASP-recommended
//! profile (m=19456 KiB, t=2, p=1).

use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

use crate::error::{AppError, AppResult};

pub const MIN_PASSWORD_LEN: usize = 8;

/// A valid argon2id hash of a value no one knows.
///
/// Verified against when the submitted username does not exist, so that a
/// missing account costs the same time as a wrong password and the two cannot
/// be told apart by timing.
static DUMMY_HASH: std::sync::OnceLock<String> = std::sync::OnceLock::new();

pub fn validate_password_len(password: &str) -> AppResult<()> {
    if password.chars().count() < MIN_PASSWORD_LEN {
        return Err(AppError::bad_request(format!(
            "password must be at least {MIN_PASSWORD_LEN} characters"
        )));
    }
    Ok(())
}

pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::internal(format!("password hashing failed: {e}")))
}

/// Verifies a password against a stored hash.
///
/// A malformed stored hash verifies as `false` rather than erroring; it cannot
/// be a successful login either way, and treating it as an error would leak
/// which accounts have corrupt rows.
pub fn verify_password(password: &str, hash: &str) -> bool {
    match PasswordHash::new(hash) {
        Ok(parsed) => Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok(),
        Err(_) => {
            tracing::warn!("stored password hash could not be parsed");
            false
        }
    }
}

/// Burns the same work a real verification would, for logins against unknown
/// usernames.
pub fn verify_against_dummy(password: &str) {
    let hash = DUMMY_HASH.get_or_init(|| {
        hash_password("grid-reveal-nonexistent-account-placeholder")
            .unwrap_or_else(|_| String::new())
    });
    if !hash.is_empty() {
        let _ = verify_password(password, hash);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_round_trips() {
        let hash = hash_password("correct horse battery").unwrap();
        assert!(verify_password("correct horse battery", &hash));
        assert!(!verify_password("wrong password", &hash));
    }

    #[test]
    fn hashes_are_salted_so_two_hashes_of_one_password_differ() {
        let a = hash_password("same password").unwrap();
        let b = hash_password("same password").unwrap();
        assert_ne!(a, b);
        assert!(verify_password("same password", &a));
        assert!(verify_password("same password", &b));
    }

    #[test]
    fn hash_is_argon2id() {
        let hash = hash_password("whatever").unwrap();
        assert!(hash.starts_with("$argon2id$"), "got {hash}");
    }

    #[test]
    fn malformed_hash_fails_closed() {
        assert!(!verify_password("anything", "not-a-hash"));
        assert!(!verify_password("anything", ""));
    }

    #[test]
    fn short_passwords_are_rejected() {
        assert!(validate_password_len("1234567").is_err());
        assert!(validate_password_len("12345678").is_ok());
        // Length counts characters, not bytes.
        assert!(validate_password_len("ねこねこねこねこ").is_ok());
    }
}
