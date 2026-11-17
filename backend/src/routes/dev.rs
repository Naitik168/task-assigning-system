//! Development-only helpers (mounted only when APP_ENV=development).

use axum::{
    extract::{Query, State},
    Json,
};

use crate::{
    dto::{EmailLogResponse, LatestEmailQuery, ResetResponse},
    error::{AppError, AppResult, ErrorBody},
    repositories::{challenges, email_logs, tasks, users},
    services::cache::invalidate_my_tasks,
    state::AppState,
};

/// Latest email from the dev outbox (contains the 2FA code).
#[utoipa::path(get, path = "/dev/email-logs/latest", tag = "dev", params(LatestEmailQuery),
    responses((status = 200, body = EmailLogResponse), (status = 404, body = ErrorBody)))]
pub async fn latest_email(
    State(state): State<AppState>,
    Query(query): Query<LatestEmailQuery>,
) -> AppResult<Json<EmailLogResponse>> {
    let email = email_logs::latest(&state.db, query.email.as_deref())
        .await?
        .ok_or_else(|| AppError::NotFound("No emails have been sent yet.".into()))?;
    Ok(Json(email.into()))
}

/// Deletes all tasks, challenges and email logs (users are kept) and clears
/// every user's cache, so the validation flow can be re-run from a clean state.
#[utoipa::path(post, path = "/dev/reset", tag = "dev", responses((status = 200, body = ResetResponse)))]
pub async fn reset(State(state): State<AppState>) -> AppResult<Json<ResetResponse>> {
    let mut tx = state.db.begin().await?;
    let deleted_tasks = tasks::delete_all(&mut *tx).await?;
    let deleted_challenges = challenges::delete_all(&mut *tx).await?;
    let deleted_emails = email_logs::delete_all(&mut *tx).await?;
    tx.commit().await?;

    let user_ids = users::list_ids(&state.db).await?;
    invalidate_my_tasks(state.cache.as_ref(), user_ids).await;

    Ok(Json(ResetResponse {
        deleted_tasks,
        deleted_challenges,
        deleted_emails,
    }))
}
