//! Request/response contracts (the public API shape).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;
use validator::Validate;

use crate::models::{EmailLog, Role, TaskDetails, TaskPriority, TaskStatus, User};

// ---------- users ----------

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserResponse {
    pub id: Uuid,
    pub full_name: String,
    pub email: String,
    pub role: Role,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResponse {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            full_name: u.full_name,
            email: u.email,
            role: u.role,
            created_at: u.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserSummary {
    pub email: String,
    pub role: Role,
}

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListUsersQuery {
    /// Optional role filter, e.g. `staff`.
    pub role: Option<Role>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SeededUser {
    #[serde(flatten)]
    pub user: UserResponse,
    /// `false` when the user already existed (seeding is idempotent).
    pub created: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SeedUsersResponse {
    pub users: Vec<SeededUser>,
}

// ---------- auth ----------

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct LoginRequest {
    #[validate(email(message = "email must be a valid email address"))]
    #[schema(example = "admin@example.com")]
    pub email: String,
    #[validate(length(min = 1, max = 256, message = "password is required"))]
    #[schema(example = "Admin@12345")]
    pub password: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct LoginResponse {
    pub login_challenge_id: Uuid,
    pub message: String,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct VerifyTwoFactorRequest {
    pub login_challenge_id: Uuid,
    #[validate(length(equal = 6, message = "code must be exactly 6 digits"))]
    #[schema(example = "123456")]
    pub code: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in_seconds: u64,
    pub user: UserResponse,
}

// ---------- dev ----------

#[derive(Debug, Deserialize, IntoParams)]
pub struct LatestEmailQuery {
    /// Filter by recipient. Defaults to the most recent email for anyone.
    pub email: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct EmailLogResponse {
    pub id: Uuid,
    pub to_email: String,
    pub subject: String,
    pub body: String,
    /// The 6-digit code parsed from the body, for convenience in dev tooling.
    pub verification_code: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<EmailLog> for EmailLogResponse {
    fn from(e: EmailLog) -> Self {
        let verification_code = extract_code(&e.body);
        Self {
            id: e.id,
            to_email: e.to_email,
            subject: e.subject,
            body: e.body,
            verification_code,
            created_at: e.created_at,
        }
    }
}

/// Returns the first run of exactly six ASCII digits in `text`.
pub fn extract_code(text: &str) -> Option<String> {
    text.split(|c: char| !c.is_ascii_digit())
        .find(|chunk| chunk.len() == 6)
        .map(str::to_string)
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ResetResponse {
    pub deleted_tasks: u64,
    pub deleted_challenges: u64,
    pub deleted_emails: u64,
}

// ---------- tasks ----------

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct CreateTaskRequest {
    #[validate(length(min = 1, max = 200, message = "title must be 1-200 characters"))]
    #[schema(example = "Infiltrate SPECTRE HQ")]
    pub title: String,
    #[validate(length(max = 2000, message = "description must be at most 2000 characters"))]
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct UpdateTaskRequest {
    #[validate(length(min = 1, max = 200, message = "title must be 1-200 characters"))]
    pub title: Option<String>,
    #[validate(length(max = 2000, message = "description must be at most 2000 characters"))]
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
}

impl UpdateTaskRequest {
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.status.is_none()
            && self.priority.is_none()
    }
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct AssignTasksRequest {
    #[validate(length(min = 1, max = 100, message = "task_ids must contain 1-100 ids"))]
    pub task_ids: Vec<Uuid>,
    #[validate(email(message = "assignee_email must be a valid email address"))]
    #[schema(example = "jamesbond@example.com")]
    pub assignee_email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskResponse {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub created_by: String,
    pub assigned_to: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<TaskDetails> for TaskResponse {
    fn from(d: TaskDetails) -> Self {
        Self {
            id: d.task.id,
            title: d.task.title,
            description: d.task.description,
            status: d.task.status,
            priority: d.task.priority,
            created_by: d.created_by_email,
            assigned_to: d.assigned_to_email,
            created_at: d.task.created_at,
            updated_at: d.task.updated_at,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AssignTasksResponse {
    pub assignee: UserSummary,
    pub assigned_count: usize,
    pub tasks: Vec<TaskResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TaskSummary {
    pub total_assigned_tasks: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema)]
pub struct CacheMeta {
    pub hit: bool,
}

/// What is stored in the cache: the response minus the `cache` block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyTasksPayload {
    pub user: UserSummary,
    pub tasks: Vec<TaskResponse>,
    pub summary: TaskSummary,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MyTasksResponse {
    pub user: UserSummary,
    pub tasks: Vec<TaskResponse>,
    pub summary: TaskSummary,
    pub cache: CacheMeta,
}

impl MyTasksPayload {
    pub fn into_response(self, hit: bool) -> MyTasksResponse {
        MyTasksResponse {
            user: self.user,
            tasks: self.tasks,
            summary: self.summary,
            cache: CacheMeta { hit },
        }
    }
}

// ---------- misc ----------

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub database: String,
    pub cache_backend: String,
    pub cache: String,
}

#[cfg(test)]
mod tests {
    use super::extract_code;

    #[test]
    fn extracts_six_digit_code() {
        assert_eq!(
            extract_code("Your code is 042917. It expires in 5 minutes."),
            Some("042917".into())
        );
        assert_eq!(extract_code("no code 12345 here 1234567"), None);
    }
}
