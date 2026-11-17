use std::collections::HashSet;

use uuid::Uuid;

use crate::{
    dto::{
        AssignTasksRequest, AssignTasksResponse, CreateTaskRequest, MyTasksPayload,
        MyTasksResponse, TaskResponse, TaskSummary, UpdateTaskRequest, UserSummary,
    },
    error::{AppError, AppResult},
    middleware::auth::AuthUser,
    models::{Role, TaskPriority, TaskStatus},
    repositories::{
        tasks::{self, NewTask, TaskChanges},
        users,
    },
    services::cache::{invalidate_my_tasks, my_tasks_key},
    state::AppState,
};

pub async fn create_task(
    state: &AppState,
    creator: &AuthUser,
    req: CreateTaskRequest,
) -> AppResult<TaskResponse> {
    let title = req.title.trim();
    if title.is_empty() {
        return Err(AppError::Validation("title must not be blank".into()));
    }
    let description = req
        .description
        .as_deref()
        .map(str::trim)
        .unwrap_or_default();

    let task = tasks::insert(
        &state.db,
        NewTask {
            title,
            description,
            status: req.status.unwrap_or(TaskStatus::Todo),
            priority: req.priority.unwrap_or(TaskPriority::Medium),
            created_by_id: creator.id,
        },
    )
    .await?;
    tracing::info!(task_id = %task.task.id, by = %creator.email, "task created");
    Ok(task.into())
}

pub async fn list_tasks(state: &AppState) -> AppResult<Vec<TaskResponse>> {
    Ok(tasks::list_all(&state.db)
        .await?
        .into_iter()
        .map(Into::into)
        .collect())
}

pub async fn assign_tasks(
    state: &AppState,
    req: AssignTasksRequest,
) -> AppResult<AssignTasksResponse> {
    // De-duplicate while keeping the caller's order.
    let mut seen = HashSet::new();
    let task_ids: Vec<Uuid> = req
        .task_ids
        .into_iter()
        .filter(|id| seen.insert(*id))
        .collect();

    let assignee = users::find_by_email(&state.db, &req.assignee_email)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("No user with email {}", req.assignee_email)))?;
    if assignee.role != Role::Staff {
        return Err(AppError::Validation(
            "Tasks can only be assigned to staff users".into(),
        ));
    }

    let mut tx = state.db.begin().await?;
    let current = tasks::lock_assignees(&mut *tx, &task_ids).await?;
    if current.len() != task_ids.len() {
        let found: HashSet<Uuid> = current.iter().map(|(id, _)| *id).collect();
        let missing: Vec<String> = task_ids
            .iter()
            .filter(|id| !found.contains(id))
            .map(Uuid::to_string)
            .collect();
        return Err(AppError::NotFound(format!(
            "Tasks not found: {}",
            missing.join(", ")
        )));
    }
    tasks::set_assignee(&mut *tx, &task_ids, assignee.id).await?;
    tx.commit().await?;

    // Invalidate after commit: the new assignee and anyone who lost a task.
    let affected = current
        .iter()
        .filter_map(|(_, prev)| *prev)
        .chain(std::iter::once(assignee.id));
    invalidate_my_tasks(state.cache.as_ref(), affected).await;

    let updated: Vec<TaskResponse> = tasks::find_by_ids(&state.db, &task_ids)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    tracing::info!(assignee = %assignee.email, count = updated.len(), "tasks assigned");

    Ok(AssignTasksResponse {
        assignee: UserSummary {
            email: assignee.email,
            role: assignee.role,
        },
        assigned_count: updated.len(),
        tasks: updated,
    })
}

pub async fn update_task(
    state: &AppState,
    id: Uuid,
    req: UpdateTaskRequest,
) -> AppResult<TaskResponse> {
    if req.is_empty() {
        return Err(AppError::Validation(
            "Provide at least one field to update".into(),
        ));
    }
    let title = req.title.as_deref().map(str::trim);
    if title == Some("") {
        return Err(AppError::Validation("title must not be blank".into()));
    }
    let updated = tasks::update(
        &state.db,
        id,
        TaskChanges {
            title,
            description: req.description.as_deref().map(str::trim),
            status: req.status,
            priority: req.priority,
        },
    )
    .await?
    .ok_or_else(|| AppError::NotFound(format!("Task {id} not found")))?;

    if let Some(assignee) = updated.task.assigned_to_id {
        invalidate_my_tasks(state.cache.as_ref(), [assignee]).await;
    }
    Ok(updated.into())
}

/// Cache-aside read: cache hit -> `cache.hit = true`; miss -> DB, then store.
pub async fn view_my_tasks(state: &AppState, user: &AuthUser) -> AppResult<MyTasksResponse> {
    let key = my_tasks_key(user.id);

    match state.cache.get(&key).await {
        Ok(Some(raw)) => match serde_json::from_str::<MyTasksPayload>(&raw) {
            Ok(payload) => return Ok(payload.into_response(true)),
            Err(e) => tracing::warn!(error = %e, %key, "corrupt cache entry, reloading"),
        },
        Ok(None) => {}
        Err(e) => tracing::warn!(error = %e, %key, "cache read failed, using database"),
    }

    let db_user = users::find_by_id(&state.db, user.id)
        .await?
        .ok_or_else(|| AppError::Unauthorized("User no longer exists.".into()))?;
    let tasks: Vec<TaskResponse> = tasks::list_assigned_to(&state.db, user.id)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    let payload = MyTasksPayload {
        user: UserSummary {
            email: db_user.email,
            role: db_user.role,
        },
        summary: TaskSummary {
            total_assigned_tasks: tasks.len(),
        },
        tasks,
    };

    match serde_json::to_string(&payload) {
        Ok(raw) => {
            if let Err(e) = state.cache.set(&key, raw, state.config.cache_ttl).await {
                tracing::warn!(error = %e, %key, "cache write failed");
            }
        }
        Err(e) => tracing::warn!(error = %e, "failed to serialise cache payload"),
    }
    Ok(payload.into_response(false))
}
