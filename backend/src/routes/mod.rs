use axum::{
    http::{header, HeaderValue, Method},
    routing::{get, patch, post},
    Router,
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::state::AppState;

pub mod auth;
pub mod dev;
pub mod docs;
pub mod health;
pub mod seed;
pub mod tasks;
pub mod users;

pub fn build_router(state: AppState) -> Router {
    let mut router = Router::new()
        .route("/health", get(health::health))
        .route("/auth/login", post(auth::login))
        .route("/auth/verify-2fa", post(auth::verify_two_factor))
        .route("/auth/me", get(auth::me))
        .route("/users", get(users::list_users))
        .route("/tasks", post(tasks::create_task).get(tasks::list_tasks))
        .route("/tasks/assign", post(tasks::assign_tasks))
        .route("/tasks/view-my-tasks", get(tasks::view_my_tasks))
        .route("/tasks/{id}", patch(tasks::update_task))
        .route("/openapi.json", get(docs::openapi_json))
        .route("/docs", get(docs::swagger_ui));

    // Seed + email-log + reset endpoints exist only in development.
    if state.config.is_development() {
        router = router
            .route("/seed/users", post(seed::seed_users))
            .route("/dev/email-logs/latest", get(dev::latest_email))
            .route("/dev/reset", post(dev::reset));
    }

    router
        .layer(cors_layer(&state.config.cors_origins))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn cors_layer(origins: &[String]) -> CorsLayer {
    let origins: Vec<HeaderValue> = origins.iter().filter_map(|o| o.parse().ok()).collect();
    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::OPTIONS])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
}
