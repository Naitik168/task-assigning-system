use axum::{
    extract::{rejection::QueryRejection, Query, State},
    Json,
};

use crate::{
    dto::{ListUsersQuery, UserResponse},
    error::{AppError, AppResult, ErrorBody},
    middleware::auth::AdminUser,
    repositories::users,
    state::AppState,
};

/// List users, optionally filtered by role (admin only; used for the assignee picker).
#[utoipa::path(get, path = "/users", tag = "users", params(ListUsersQuery), security(("bearer_auth" = [])),
    responses((status = 200, body = [UserResponse]), (status = 403, body = ErrorBody)))]
pub async fn list_users(
    State(state): State<AppState>,
    _admin: AdminUser,
    query: Result<Query<ListUsersQuery>, QueryRejection>,
) -> AppResult<Json<Vec<UserResponse>>> {
    let Query(query) = query.map_err(|e| AppError::Validation(e.body_text()))?;
    let list = users::list(&state.db, query.role).await?;
    Ok(Json(list.into_iter().map(Into::into).collect()))
}
