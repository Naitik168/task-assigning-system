use utoipa::{
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
    Modify, OpenApi,
};

use crate::routes;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Task Manager API",
        version = "0.1.0",
        description = "Task management with email 2FA, JWT, role-based access and per-user caching."
    ),
    paths(
        routes::health::health,
        routes::seed::seed_users,
        routes::auth::login,
        routes::auth::verify_two_factor,
        routes::auth::me,
        routes::dev::latest_email,
        routes::dev::reset,
        routes::users::list_users,
        routes::tasks::create_task,
        routes::tasks::list_tasks,
        routes::tasks::assign_tasks,
        routes::tasks::update_task,
        routes::tasks::view_my_tasks,
    ),
    modifiers(&BearerAuth),
    tags(
        (name = "auth", description = "Password + email 2FA login"),
        (name = "tasks", description = "Task management"),
        (name = "users", description = "User directory"),
        (name = "dev", description = "Development-only helpers"),
        (name = "system", description = "Health")
    )
)]
pub struct ApiDoc;

struct BearerAuth;

impl Modify for BearerAuth {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}
