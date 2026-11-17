use axum::{extract::State, Json};

use crate::{
    dto::{LoginRequest, LoginResponse, TokenResponse, UserResponse, VerifyTwoFactorRequest},
    error::{AppError, AppResult, ErrorBody},
    extract::ValidatedJson,
    middleware::auth::AuthUser,
    repositories::users,
    services::auth,
    state::AppState,
};

/// Step 1: validate email + password, create a 2FA challenge and email a code.
/// Does NOT return a JWT.
#[utoipa::path(post, path = "/auth/login", tag = "auth", request_body = LoginRequest,
    responses(
        (status = 200, body = LoginResponse),
        (status = 401, description = "Invalid credentials", body = ErrorBody),
        (status = 422, body = ErrorBody)))]
pub async fn login(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    Ok(Json(auth::start_login(&state, req).await?))
}

/// Step 2: exchange the challenge id + emailed code for a JWT.
#[utoipa::path(post, path = "/auth/verify-2fa", tag = "auth", request_body = VerifyTwoFactorRequest,
    responses(
        (status = 200, body = TokenResponse),
        (status = 401, description = "INVALID_CODE / CODE_EXPIRED / CODE_ALREADY_USED / TOO_MANY_ATTEMPTS", body = ErrorBody),
        (status = 422, body = ErrorBody)))]
pub async fn verify_two_factor(
    State(state): State<AppState>,
    ValidatedJson(req): ValidatedJson<VerifyTwoFactorRequest>,
) -> AppResult<Json<TokenResponse>> {
    Ok(Json(auth::verify_two_factor(&state, req).await?))
}

/// Current user profile (used by the frontend to restore a session).
#[utoipa::path(get, path = "/auth/me", tag = "auth", security(("bearer_auth" = [])),
    responses((status = 200, body = UserResponse), (status = 401, body = ErrorBody)))]
pub async fn me(State(state): State<AppState>, user: AuthUser) -> AppResult<Json<UserResponse>> {
    let user = users::find_by_id(&state.db, user.id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("User no longer exists.".into()))?;
    Ok(Json(user.into()))
}
