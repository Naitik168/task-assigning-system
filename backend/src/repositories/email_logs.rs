use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::EmailLog;

pub async fn insert<'e>(
    ex: impl PgExecutor<'e>,
    to_email: &str,
    subject: &str,
    body: &str,
) -> sqlx::Result<EmailLog> {
    sqlx::query_as::<_, EmailLog>(
        "INSERT INTO email_logs (id, to_email, subject, body)
         VALUES ($1, $2, $3, $4)
         RETURNING id, to_email, subject, body, created_at",
    )
    .bind(Uuid::new_v4())
    .bind(to_email)
    .bind(subject)
    .bind(body)
    .fetch_one(ex)
    .await
}

pub async fn latest<'e>(
    ex: impl PgExecutor<'e>,
    to_email: Option<&str>,
) -> sqlx::Result<Option<EmailLog>> {
    sqlx::query_as::<_, EmailLog>(
        "SELECT id, to_email, subject, body, created_at FROM email_logs
         WHERE ($1::text IS NULL OR to_email = $1)
         ORDER BY created_at DESC
         LIMIT 1",
    )
    .bind(to_email.map(|e| e.trim().to_lowercase()))
    .fetch_optional(ex)
    .await
}

pub async fn delete_all<'e>(ex: impl PgExecutor<'e>) -> sqlx::Result<u64> {
    Ok(sqlx::query("DELETE FROM email_logs")
        .execute(ex)
        .await?
        .rows_affected())
}
