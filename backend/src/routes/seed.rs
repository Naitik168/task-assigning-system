use axum::{extract::State, Json};

use crate::{dto::SeedUsersResponse, error::AppResult, services::seed, state::AppState};

/// Create the Admin and James Bond users (idempotent, development only).
#[utoipa::path(post, path = "/seed/users", tag = "dev",
    responses((status = 200, description = "Users created or already present", body = SeedUsersResponse)))]
pub async fn seed_users(State(state): State<AppState>) -> AppResult<Json<SeedUsersResponse>> {
    Ok(Json(seed::seed_users(&state).await?))
}
