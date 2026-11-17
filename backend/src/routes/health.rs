use axum::{extract::State, http::StatusCode, Json};

use crate::{dto::HealthResponse, state::AppState};

/// Liveness/readiness probe used by Docker health checks.
#[utoipa::path(get, path = "/health", tag = "system",
    responses((status = 200, body = HealthResponse), (status = 503, body = HealthResponse)))]
pub async fn health(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let db_ok = sqlx::query("SELECT 1").execute(&state.db).await.is_ok();
    let cache_ok = state.cache.ping().await.is_ok();
    let status = if db_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(HealthResponse {
            status: if db_ok { "ok" } else { "degraded" }.into(),
            database: if db_ok { "ok" } else { "unreachable" }.into(),
            cache_backend: state.cache.backend_name().into(),
            cache: if cache_ok { "ok" } else { "unreachable" }.into(),
        }),
    )
}
