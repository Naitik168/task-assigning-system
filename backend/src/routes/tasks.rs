use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use uuid::Uuid;

use crate::{
    dto::{
        AssignTasksRequest, AssignTasksResponse, CreateTaskRequest, MyTasksResponse, TaskResponse,
        UpdateTaskRequest,
    },
    error::{AppResult, ErrorBody},
    extract::ValidatedJson,
    middleware::auth::{AdminUser, AuthUser},
    services::tasks,
    state::AppState,
};

/// Create a task (admin only - staff receive 403).
#[utoipa::path(post, path = "/tasks", tag = "tasks", request_body = CreateTaskRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 201, body = TaskResponse),
        (status = 401, body = ErrorBody),
        (status = 403, description = "Caller is not an admin", body = ErrorBody),
        (status = 422, body = ErrorBody)))]
pub async fn create_task(
    State(state): State<AppState>,
    AdminUser(admin): AdminUser,
    ValidatedJson(req): ValidatedJson<CreateTaskRequest>,
) -> AppResult<(StatusCode, Json<TaskResponse>)> {
    let task = tasks::create_task(&state, &admin, req).await?;
    Ok((StatusCode::CREATED, Json(task)))
}

/// List all tasks (admin only).
#[utoipa::path(get, path = "/tasks", tag = "tasks", security(("bearer_auth" = [])),
    responses((status = 200, body = [TaskResponse]), (status = 403, body = ErrorBody)))]
pub async fn list_tasks(
    State(state): State<AppState>,
    _admin: AdminUser,
) -> AppResult<Json<Vec<TaskResponse>>> {
    Ok(Json(tasks::list_tasks(&state).await?))
}

/// Assign tasks to a staff user (admin only). Invalidates affected users' caches.
#[utoipa::path(post, path = "/tasks/assign", tag = "tasks", request_body = AssignTasksRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = AssignTasksResponse),
        (status = 403, body = ErrorBody),
        (status = 404, description = "Unknown task id or assignee", body = ErrorBody),
        (status = 422, body = ErrorBody)))]
pub async fn assign_tasks(
    State(state): State<AppState>,
    _admin: AdminUser,
    ValidatedJson(req): ValidatedJson<AssignTasksRequest>,
) -> AppResult<Json<AssignTasksResponse>> {
    Ok(Json(tasks::assign_tasks(&state, req).await?))
}

/// Update a task's fields (admin only). Invalidates the assignee's cache.
#[utoipa::path(patch, path = "/tasks/{id}", tag = "tasks", request_body = UpdateTaskRequest,
    params(("id" = Uuid, Path, description = "Task id")),
    security(("bearer_auth" = [])),
    responses(
        (status = 200, body = TaskResponse),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody)))]
pub async fn update_task(
    State(state): State<AppState>,
    _admin: AdminUser,
    Path(id): Path<Uuid>,
    ValidatedJson(req): ValidatedJson<UpdateTaskRequest>,
) -> AppResult<Json<TaskResponse>> {
    Ok(Json(tasks::update_task(&state, id, req).await?))
}

/// Tasks assigned to the caller, cached per user (`cache.hit` shows the source).
#[utoipa::path(get, path = "/tasks/view-my-tasks", tag = "tasks", security(("bearer_auth" = [])),
    responses((status = 200, body = MyTasksResponse), (status = 401, body = ErrorBody)))]
pub async fn view_my_tasks(
    State(state): State<AppState>,
    user: AuthUser,
) -> AppResult<Json<MyTasksResponse>> {
    Ok(Json(tasks::view_my_tasks(&state, &user).await?))
}
