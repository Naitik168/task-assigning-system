use chrono::{DateTime, Utc};
use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::LoginChallenge;

pub async fn insert<'e>(
    ex: impl PgExecutor<'e>,
    id: Uuid,
    user_id: Uuid,
    code_hash: &str,
    expires_at: DateTime<Utc>,
) -> sqlx::Result<()> {
    sqlx::query(
        "INSERT INTO login_challenges (id, user_id, code_hash, expires_at)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(id)
    .bind(user_id)
    .bind(code_hash)
    .bind(expires_at)
    .execute(ex)
    .await?;
    Ok(())
}

/// Invalidates any still-open challenge for the user so only the newest code works.
pub async fn supersede_open<'e>(ex: impl PgExecutor<'e>, user_id: Uuid) -> sqlx::Result<u64> {
    let result = sqlx::query(
        "UPDATE login_challenges SET consumed_at = now()
         WHERE user_id = $1 AND consumed_at IS NULL",
    )
    .bind(user_id)
    .execute(ex)
    .await?;
    Ok(result.rows_affected())
}

/// Loads a challenge and row-locks it, serialising concurrent verifications.
pub async fn find_for_update<'e>(
    ex: impl PgExecutor<'e>,
    id: Uuid,
) -> sqlx::Result<Option<LoginChallenge>> {
    sqlx::query_as::<_, LoginChallenge>(
        "SELECT id, user_id, code_hash, expires_at, consumed_at, failed_attempts, created_at,
                (expires_at <= now()) AS is_expired
         FROM login_challenges WHERE id = $1
         FOR UPDATE",
    )
    .bind(id)
    .fetch_optional(ex)
    .await
}

pub async fn increment_failed_attempts<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> sqlx::Result<i32> {
    sqlx::query_scalar(
        "UPDATE login_challenges SET failed_attempts = failed_attempts + 1
         WHERE id = $1 RETURNING failed_attempts",
    )
    .bind(id)
    .fetch_one(ex)
    .await
}

/// Marks the challenge used. Returns `false` if it was already consumed or
/// expired (guard against double-spend even without the row lock).
pub async fn mark_consumed<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> sqlx::Result<bool> {
    let result = sqlx::query(
        "UPDATE login_challenges SET consumed_at = now()
         WHERE id = $1 AND consumed_at IS NULL AND expires_at > now()",
    )
    .bind(id)
    .execute(ex)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub async fn delete_all<'e>(ex: impl PgExecutor<'e>) -> sqlx::Result<u64> {
    Ok(sqlx::query("DELETE FROM login_challenges")
        .execute(ex)
        .await?
        .rows_affected())
}
