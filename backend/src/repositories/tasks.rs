use sqlx::PgExecutor;
use uuid::Uuid;

use crate::models::{TaskDetails, TaskPriority, TaskStatus};

/// Projection shared by every read: task columns + creator/assignee emails.
/// Expects the task row to be aliased as `t`.
const DETAILS_SELECT: &str = "
    SELECT t.id, t.title, t.description, t.status, t.priority,
           t.created_by_id, t.assigned_to_id, t.created_at, t.updated_at,
           c.email AS created_by_email,
           a.email AS assigned_to_email";

const DETAILS_JOINS: &str = "
    JOIN users c ON c.id = t.created_by_id
    LEFT JOIN users a ON a.id = t.assigned_to_id";

pub struct NewTask<'a> {
    pub title: &'a str,
    pub description: &'a str,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub created_by_id: Uuid,
}

pub async fn insert<'e>(ex: impl PgExecutor<'e>, new: NewTask<'_>) -> sqlx::Result<TaskDetails> {
    sqlx::query_as::<_, TaskDetails>(&format!(
        "WITH t AS (
            INSERT INTO tasks (id, title, description, status, priority, created_by_id)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
         )
         {DETAILS_SELECT} FROM t {DETAILS_JOINS}"
    ))
    .bind(Uuid::new_v4())
    .bind(new.title)
    .bind(new.description)
    .bind(new.status)
    .bind(new.priority)
    .bind(new.created_by_id)
    .fetch_one(ex)
    .await
}

pub async fn list_all<'e>(ex: impl PgExecutor<'e>) -> sqlx::Result<Vec<TaskDetails>> {
    sqlx::query_as::<_, TaskDetails>(&format!(
        "{DETAILS_SELECT} FROM tasks t {DETAILS_JOINS} ORDER BY t.created_at, t.id"
    ))
    .fetch_all(ex)
    .await
}

/// Tasks assigned to `user_id`, highest priority first.
pub async fn list_assigned_to<'e>(
    ex: impl PgExecutor<'e>,
    user_id: Uuid,
) -> sqlx::Result<Vec<TaskDetails>> {
    sqlx::query_as::<_, TaskDetails>(&format!(
        "{DETAILS_SELECT} FROM tasks t {DETAILS_JOINS}
         WHERE t.assigned_to_id = $1
         ORDER BY t.priority DESC, t.created_at, t.id"
    ))
    .bind(user_id)
    .fetch_all(ex)
    .await
}

pub async fn find_by_ids<'e>(
    ex: impl PgExecutor<'e>,
    ids: &[Uuid],
) -> sqlx::Result<Vec<TaskDetails>> {
    sqlx::query_as::<_, TaskDetails>(&format!(
        "{DETAILS_SELECT} FROM tasks t {DETAILS_JOINS}
         WHERE t.id = ANY($1)
         ORDER BY t.priority DESC, t.created_at, t.id"
    ))
    .bind(ids)
    .fetch_all(ex)
    .await
}

/// Locks the rows (FOR UPDATE) and returns `(task_id, current_assignee)`.
pub async fn lock_assignees<'e>(
    ex: impl PgExecutor<'e>,
    ids: &[Uuid],
) -> sqlx::Result<Vec<(Uuid, Option<Uuid>)>> {
    sqlx::query_as("SELECT id, assigned_to_id FROM tasks WHERE id = ANY($1) FOR UPDATE")
        .bind(ids)
        .fetch_all(ex)
        .await
}

pub async fn set_assignee<'e>(
    ex: impl PgExecutor<'e>,
    ids: &[Uuid],
    assignee_id: Uuid,
) -> sqlx::Result<u64> {
    let result =
        sqlx::query("UPDATE tasks SET assigned_to_id = $2, updated_at = now() WHERE id = ANY($1)")
            .bind(ids)
            .bind(assignee_id)
            .execute(ex)
            .await?;
    Ok(result.rows_affected())
}

pub struct TaskChanges<'a> {
    pub title: Option<&'a str>,
    pub description: Option<&'a str>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
}

/// Partial update; `None` fields keep their current value.
pub async fn update<'e>(
    ex: impl PgExecutor<'e>,
    id: Uuid,
    changes: TaskChanges<'_>,
) -> sqlx::Result<Option<TaskDetails>> {
    sqlx::query_as::<_, TaskDetails>(&format!(
        "WITH t AS (
            UPDATE tasks SET
                title       = COALESCE($2, title),
                description = COALESCE($3, description),
                status      = COALESCE($4, status),
                priority    = COALESCE($5, priority),
                updated_at  = now()
            WHERE id = $1
            RETURNING *
         )
         {DETAILS_SELECT} FROM t {DETAILS_JOINS}"
    ))
    .bind(id)
    .bind(changes.title)
    .bind(changes.description)
    .bind(changes.status)
    .bind(changes.priority)
    .fetch_optional(ex)
    .await
}

pub async fn delete_all<'e>(ex: impl PgExecutor<'e>) -> sqlx::Result<u64> {
    Ok(sqlx::query("DELETE FROM tasks")
        .execute(ex)
        .await?
        .rows_affected())
}
