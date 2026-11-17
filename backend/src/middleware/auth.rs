//! Authentication / authorisation extractors.
//!
//! * `AuthUser`  - any valid Bearer JWT, else 401.
//! * `AdminUser` - valid JWT with role `admin`, else 403.
//!
//! Using extractors (instead of a router-wide layer) makes each handler's
//! access requirement visible in its signature.

use axum::{extract::FromRequestParts, http::request::Parts};
use uuid::Uuid;

use crate::{error::AppError, models::Role, services::jwt, state::AppState};

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub email: String,
    pub role: Role,
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                AppError::Unauthorized("Missing Authorization: Bearer <token> header.".into())
            })?;

        let token = header
            .split_once(' ')
            .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
            .map(|(_, token)| token.trim())
            .filter(|t| !t.is_empty())
            .ok_or_else(|| {
                AppError::Unauthorized("Authorization header must use the Bearer scheme.".into())
            })?;

        let claims = jwt::decode_token(&state.config.jwt_secret, token)
            .map_err(|_| AppError::Unauthorized("Invalid or expired token.".into()))?;

        Ok(Self {
            id: claims.sub,
            email: claims.email,
            role: claims.role,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AdminUser(pub AuthUser);

impl FromRequestParts<AppState> for AdminUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.role != Role::Admin {
            tracing::warn!(user = %user.email, path = %parts.uri.path(), "forbidden: admin role required");
            return Err(AppError::Forbidden(
                "Only admins can perform this action.".into(),
            ));
        }
        Ok(Self(user))
    }
}
