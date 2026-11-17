//! Login (step 1: password -> 2FA challenge) and verification (step 2: code -> JWT).

use chrono::Utc;
use uuid::Uuid;

use crate::{
    dto::{LoginRequest, LoginResponse, TokenResponse, VerifyTwoFactorRequest},
    error::{AppError, AppResult, TwoFactorError},
    repositories::{challenges, users},
    services::{jwt, mailer::OutgoingEmail, otp, password},
    state::AppState,
};

pub async fn start_login(state: &AppState, req: LoginRequest) -> AppResult<LoginResponse> {
    let user = users::find_by_email(&state.db, &req.email).await?;

    // Always run Argon2 so unknown emails and wrong passwords look identical.
    let valid = password::verify_password(
        req.password,
        user.as_ref().map(|u| u.hashed_password.clone()),
    )
    .await?;
    let user = match (valid, user) {
        (true, Some(user)) => user,
        _ => return Err(AppError::InvalidCredentials),
    };

    let challenge_id = Uuid::new_v4();
    let code = otp::generate_code();
    let code_hash = otp::hash_code(&state.config.otp_secret, challenge_id, &code);
    let ttl = state.config.otp_ttl;
    let expires_at = Utc::now()
        + chrono::Duration::from_std(ttl).map_err(|e| AppError::Internal(e.to_string()))?;

    let mut tx = state.db.begin().await?;
    challenges::supersede_open(&mut *tx, user.id).await?;
    challenges::insert(&mut *tx, challenge_id, user.id, &code_hash, expires_at).await?;
    tx.commit().await?;

    let minutes = ttl.as_secs().div_ceil(60);
    state
        .mailer
        .send(OutgoingEmail {
            to: user.email.clone(),
            subject: "Your Task Manager verification code".into(),
            body: format!(
                "Hi {}, your verification code is {code}. It expires in {minutes} minutes and can only be used once.",
                user.full_name
            ),
        })
        .await?;

    tracing::info!(user_id = %user.id, %challenge_id, "2FA challenge created");

    Ok(LoginResponse {
        login_challenge_id: challenge_id,
        message: "A verification code has been sent to your email.".into(),
        expires_in_seconds: ttl.as_secs(),
    })
}

pub async fn verify_two_factor(
    state: &AppState,
    req: VerifyTwoFactorRequest,
) -> AppResult<TokenResponse> {
    if !otp::is_well_formed(&req.code) {
        return Err(AppError::Validation("code must be exactly 6 digits".into()));
    }

    // The row lock (FOR UPDATE) serialises concurrent attempts on the same
    // challenge, so a code can never be redeemed twice.
    let mut tx = state.db.begin().await?;
    let challenge = challenges::find_for_update(&mut *tx, req.login_challenge_id)
        .await?
        .ok_or(AppError::TwoFactor(TwoFactorError::ChallengeNotFound))?;

    if challenge.consumed_at.is_some() {
        return Err(AppError::TwoFactor(TwoFactorError::AlreadyUsed));
    }
    if challenge.is_expired {
        return Err(AppError::TwoFactor(TwoFactorError::Expired));
    }
    if challenge.failed_attempts >= state.config.otp_max_attempts {
        return Err(AppError::TwoFactor(TwoFactorError::TooManyAttempts));
    }

    if !otp::verify_code(
        &state.config.otp_secret,
        challenge.id,
        &req.code,
        &challenge.code_hash,
    ) {
        let attempts = challenges::increment_failed_attempts(&mut *tx, challenge.id).await?;
        tx.commit().await?;
        tracing::warn!(challenge_id = %challenge.id, attempts, "invalid 2FA code");
        return Err(AppError::TwoFactor(
            if attempts >= state.config.otp_max_attempts {
                TwoFactorError::TooManyAttempts
            } else {
                TwoFactorError::InvalidCode
            },
        ));
    }

    if !challenges::mark_consumed(&mut *tx, challenge.id).await? {
        return Err(AppError::TwoFactor(TwoFactorError::AlreadyUsed));
    }
    let user = users::find_by_id(&mut *tx, challenge.user_id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("User no longer exists.".into()))?;
    tx.commit().await?;

    let ttl = state.config.jwt_ttl;
    let access_token = jwt::issue_token(&state.config.jwt_secret, ttl, &user)?;
    tracing::info!(user_id = %user.id, "2FA verified, JWT issued");

    Ok(TokenResponse {
        access_token,
        token_type: "Bearer".into(),
        expires_in_seconds: ttl.as_secs(),
        user: user.into(),
    })
}
