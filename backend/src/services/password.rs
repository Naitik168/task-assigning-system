//! Argon2id password hashing. Hashing is CPU-bound, so callers run it on the
//! blocking thread pool to avoid stalling the async runtime.

use std::sync::LazyLock;

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use crate::error::{AppError, AppResult};

/// Hash used when the email doesn't exist, so the response time for
/// "unknown email" matches "wrong password" (prevents user enumeration).
static DUMMY_HASH: LazyLock<String> =
    LazyLock::new(|| hash_password_sync("dummy-password-for-timing").expect("dummy hash"));

pub fn hash_password_sync(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(format!("password hashing failed: {e}")))
}

pub fn verify_password_sync(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|parsed| {
            Argon2::default()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
        .unwrap_or(false)
}

pub async fn hash_password(password: String) -> AppResult<String> {
    tokio::task::spawn_blocking(move || hash_password_sync(&password))
        .await
        .map_err(|e| AppError::Internal(format!("hash task failed: {e}")))?
}

/// Verifies `password` against `hash`, or against a dummy hash when the user
/// was not found (always returns `false` in that case).
pub async fn verify_password(password: String, hash: Option<String>) -> AppResult<bool> {
    tokio::task::spawn_blocking(move || match hash {
        Some(h) => verify_password_sync(&password, &h),
        None => {
            let _ = verify_password_sync(&password, &DUMMY_HASH);
            false
        }
    })
    .await
    .map_err(|e| AppError::Internal(format!("verify task failed: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_are_salted_and_verifiable() {
        let a = hash_password_sync("s3cret!").unwrap();
        let b = hash_password_sync("s3cret!").unwrap();
        assert_ne!(a, b, "salt must differ");
        assert!(a.starts_with("$argon2id$"));
        assert!(verify_password_sync("s3cret!", &a));
        assert!(!verify_password_sync("wrong", &a));
    }
}
