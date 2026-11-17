use crate::{
    dto::{SeedUsersResponse, SeededUser},
    error::{AppError, AppResult},
    models::Role,
    repositories::users,
    services::password,
    state::AppState,
};

pub const ADMIN_EMAIL: &str = "admin@example.com";
pub const STAFF_EMAIL: &str = "jamesbond@example.com";

/// Idempotently creates the two validation users. Existing users are left
/// untouched (their passwords are not reset).
pub async fn seed_users(state: &AppState) -> AppResult<SeedUsersResponse> {
    let specs = [
        (
            "Admin",
            ADMIN_EMAIL,
            state.config.seed_admin_password.clone(),
            Role::Admin,
        ),
        (
            "James Bond",
            STAFF_EMAIL,
            state.config.seed_staff_password.clone(),
            Role::Staff,
        ),
    ];

    let mut seeded = Vec::with_capacity(specs.len());
    for (full_name, email, plain_password, role) in specs {
        if let Some(existing) = users::find_by_email(&state.db, email).await? {
            seeded.push(SeededUser {
                user: existing.into(),
                created: false,
            });
            continue;
        }
        let hash = password::hash_password(plain_password).await?;
        let user = match users::insert_if_absent(&state.db, full_name, email, &hash, role).await? {
            Some(u) => SeededUser {
                user: u.into(),
                created: true,
            },
            // Lost a race with a concurrent seed: read the winner.
            None => SeededUser {
                user: users::find_by_email(&state.db, email)
                    .await?
                    .ok_or_else(|| AppError::Internal("seed race".into()))?
                    .into(),
                created: false,
            },
        };
        seeded.push(user);
    }
    Ok(SeedUsersResponse { users: seeded })
}
