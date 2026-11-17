//! One-time verification codes.
//!
//! Codes are 6 random digits from the OS CSPRNG. Only
//! `HMAC-SHA256(server_secret, "{challenge_id}:{code}")` is persisted: binding
//! the challenge id means a hash can't be reused across challenges, and the
//! server secret means a leaked table can't be brute-forced offline.

use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, Rng};
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

pub fn generate_code() -> String {
    format!("{:06}", OsRng.gen_range(0..1_000_000u32))
}

pub fn is_well_formed(code: &str) -> bool {
    code.len() == 6 && code.bytes().all(|b| b.is_ascii_digit())
}

fn mac(secret: &str, challenge_id: Uuid, code: &str) -> HmacSha256 {
    let mut mac =
        HmacSha256::new_from_slice(secret.as_bytes()).expect("HMAC accepts keys of any length");
    mac.update(challenge_id.as_bytes());
    mac.update(b":");
    mac.update(code.as_bytes());
    mac
}

pub fn hash_code(secret: &str, challenge_id: Uuid, code: &str) -> String {
    hex::encode(mac(secret, challenge_id, code).finalize().into_bytes())
}

/// Constant-time comparison against the stored hex digest.
pub fn verify_code(secret: &str, challenge_id: Uuid, code: &str, stored_hex: &str) -> bool {
    match hex::decode(stored_hex) {
        Ok(expected) => mac(secret, challenge_id, code)
            .verify_slice(&expected)
            .is_ok(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_codes_are_six_digits() {
        for _ in 0..200 {
            assert!(is_well_formed(&generate_code()));
        }
    }

    #[test]
    fn hash_roundtrip_and_binding() {
        let id = Uuid::new_v4();
        let h = hash_code("secret", id, "123456");
        assert_ne!(h, "123456", "code must not be stored in plain text");
        assert!(verify_code("secret", id, "123456", &h));
        assert!(!verify_code("secret", id, "654321", &h));
        assert!(!verify_code("other-secret", id, "123456", &h));
        assert!(!verify_code("secret", Uuid::new_v4(), "123456", &h));
    }

    #[test]
    fn rejects_malformed_codes() {
        assert!(!is_well_formed("12345"));
        assert!(!is_well_formed("12345a"));
        assert!(!is_well_formed("1234567"));
    }
}
