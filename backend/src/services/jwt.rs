use std::time::Duration;

use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{Role, User},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,
    pub email: String,
    pub role: Role,
    pub iat: i64,
    pub exp: i64,
}

pub fn issue_token(secret: &str, ttl: Duration, user: &User) -> AppResult<String> {
    let now = Utc::now().timestamp();
    let claims = Claims {
        sub: user.id,
        email: user.email.clone(),
        role: user.role,
        iat: now,
        exp: now + ttl.as_secs() as i64,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("failed to sign JWT: {e}")))
}

pub fn decode_token(secret: &str, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = 5;
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn user() -> User {
        User {
            id: Uuid::new_v4(),
            full_name: "James Bond".into(),
            email: "jamesbond@example.com".into(),
            hashed_password: String::new(),
            role: Role::Staff,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn roundtrip() {
        let u = user();
        let token = issue_token("secret", Duration::from_secs(60), &u).unwrap();
        let claims = decode_token("secret", &token).unwrap();
        assert_eq!(claims.sub, u.id);
        assert_eq!(claims.role, Role::Staff);
    }

    #[test]
    fn rejects_wrong_secret() {
        let token = issue_token("secret", Duration::from_secs(60), &user()).unwrap();
        assert!(decode_token("another", &token).is_err());
    }
}
