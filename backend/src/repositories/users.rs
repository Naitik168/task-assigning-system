use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::{Role, User};

const USER_COLUMNS: &str = "id, full_name, email, hashed_password, role, created_at, updated_at";

pub async fn find_by_email<'e>(ex: impl PgExecutor<'e>, email: &str) -> sqlx::Result<Option<User>> {
    sqlx::query_as::<_, User>(&format!(
        "SELECT {USER_COLUMNS} FROM users WHERE email = $1"
    ))
    .bind(email.trim().to_lowercase())
    .fetch_optional(ex)
    .await
}

pub async fn find_by_id<'e>(ex: impl PgExecutor<'e>, id: Uuid) -> sqlx::Result<Option<User>> {
    sqlx::query_as::<_, User>(&format!("SELECT {USER_COLUMNS} FROM users WHERE id = $1"))
        .bind(id)
        .fetch_optional(ex)
        .await
}

/// Inserts the user unless the email already exists. Returns `None` on conflict.
pub async fn insert_if_absent<'e>(
    ex: impl PgExecutor<'e>,
    full_name: &str,
    email: &str,
    hashed_password: &str,
    role: Role,
) -> sqlx::Result<Option<User>> {
    sqlx::query_as::<_, User>(&format!(
        "INSERT INTO users (id, full_name, email, hashed_password, role)
         VALUES ($1, $2, $3, $4, $5)
         ON CONFLICT (email) DO NOTHING
         RETURNING {USER_COLUMNS}"
    ))
    .bind(Uuid::new_v4())
    .bind(full_name)
    .bind(email.trim().to_lowercase())
    .bind(hashed_password)
    .bind(role)
    .fetch_optional(ex)
    .await
}

pub async fn list<'e>(ex: impl PgExecutor<'e>, role: Option<Role>) -> sqlx::Result<Vec<User>> {
    sqlx::query_as::<_, User>(&format!(
        "SELECT {USER_COLUMNS} FROM users
         WHERE ($1::user_role IS NULL OR role = $1)
         ORDER BY full_name"
    ))
    .bind(role)
    .fetch_all(ex)
    .await
}

pub async fn list_ids<'e>(ex: impl PgExecutor<'e>) -> sqlx::Result<Vec<Uuid>> {
    sqlx::query_scalar("SELECT id FROM users")
        .fetch_all(ex)
        .await
}
