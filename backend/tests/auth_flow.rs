mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::json;
use sqlx::PgPool;

#[sqlx::test(migrations = "./migrations")]
async fn seed_creates_admin_and_james_bond_idempotently(pool: PgPool) {
    let app = TestApp::new(pool.clone());

    let first = app.post("/seed/users", None, json!({})).await;
    assert_eq!(first.status, StatusCode::OK);
    let users = first.body["users"].as_array().unwrap();
    assert_eq!(users.len(), 2);
    assert_eq!(users[0]["email"], ADMIN_EMAIL);
    assert_eq!(users[0]["role"], "admin");
    assert_eq!(users[1]["email"], BOND_EMAIL);
    assert_eq!(users[1]["role"], "staff");
    assert!(users.iter().all(|u| u["created"] == true));
    assert!(users.iter().all(|u| u.get("hashed_password").is_none()));

    let second = app.post("/seed/users", None, json!({})).await;
    assert!(second.body["users"]
        .as_array()
        .unwrap()
        .iter()
        .all(|u| u["created"] == false));

    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM users")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2);

    let hash: String = sqlx::query_scalar("SELECT hashed_password FROM users WHERE email = $1")
        .bind(ADMIN_EMAIL)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(
        hash.starts_with("$argon2id$"),
        "passwords must be Argon2 hashed"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn login_creates_challenge_and_does_not_return_jwt(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    app.seed().await;

    let res = app
        .post(
            "/auth/login",
            None,
            json!({ "email": ADMIN_EMAIL, "password": ADMIN_PASSWORD }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);
    assert!(res.body["login_challenge_id"].is_string());
    assert_eq!(res.body["expires_in_seconds"], 300);
    assert!(
        res.body.get("access_token").is_none(),
        "login must not issue a JWT"
    );

    // An email event was recorded, and the code is not stored in plain text.
    let code = app.latest_code(ADMIN_EMAIL).await;
    let stored: String = sqlx::query_scalar("SELECT code_hash FROM login_challenges")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_ne!(stored, code);
    assert_eq!(stored.len(), 64, "HMAC-SHA256 hex digest");

    // Expiry is 5 minutes.
    let ttl: f64 = sqlx::query_scalar(
        "SELECT EXTRACT(EPOCH FROM (expires_at - created_at))::float8 FROM login_challenges",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!((299.0..=301.0).contains(&ttl), "ttl was {ttl}");
}

#[sqlx::test(migrations = "./migrations")]
async fn wrong_password_and_unknown_email_are_rejected_identically(pool: PgPool) {
    let app = TestApp::new(pool);
    app.seed().await;

    let wrong_pw = app
        .post(
            "/auth/login",
            None,
            json!({ "email": ADMIN_EMAIL, "password": "nope" }),
        )
        .await;
    let unknown = app
        .post(
            "/auth/login",
            None,
            json!({ "email": "nobody@example.com", "password": "nope" }),
        )
        .await;
    assert_eq!(wrong_pw.status, StatusCode::UNAUTHORIZED);
    assert_eq!(wrong_pw.body, unknown.body);
    assert_eq!(wrong_pw.body["error"]["code"], "INVALID_CREDENTIALS");
}

#[sqlx::test(migrations = "./migrations")]
async fn correct_code_returns_jwt(pool: PgPool) {
    let app = TestApp::new(pool);
    app.seed().await;

    let challenge = app.start_login(ADMIN_EMAIL, ADMIN_PASSWORD).await;
    let code = app.latest_code(ADMIN_EMAIL).await;
    let res = app.verify(&challenge, &code).await;

    assert_eq!(res.status, StatusCode::OK, "{:?}", res.body);
    assert_eq!(res.body["token_type"], "Bearer");
    assert_eq!(res.body["user"]["role"], "admin");
    let token = res.body["access_token"].as_str().unwrap();
    assert_eq!(token.split('.').count(), 3, "looks like a JWT");

    let me = app.get("/auth/me", Some(token)).await;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.body["email"], ADMIN_EMAIL);
}

#[sqlx::test(migrations = "./migrations")]
async fn incorrect_code_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    app.seed().await;

    let challenge = app.start_login(ADMIN_EMAIL, ADMIN_PASSWORD).await;
    let code = app.latest_code(ADMIN_EMAIL).await;

    let res = app.verify(&challenge, &wrong_code(&code)).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert_eq!(res.body["error"]["code"], "INVALID_CODE");

    // The correct code still works after a single mistake.
    assert_eq!(app.verify(&challenge, &code).await.status, StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn too_many_wrong_attempts_lock_the_challenge(pool: PgPool) {
    let app = TestApp::new(pool);
    app.seed().await;

    let challenge = app.start_login(ADMIN_EMAIL, ADMIN_PASSWORD).await;
    let code = app.latest_code(ADMIN_EMAIL).await;
    for _ in 0..5 {
        app.verify(&challenge, &wrong_code(&code)).await;
    }
    let res = app.verify(&challenge, &code).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert_eq!(res.body["error"]["code"], "TOO_MANY_ATTEMPTS");
}

#[sqlx::test(migrations = "./migrations")]
async fn expired_code_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    app.seed().await;

    let challenge = app.start_login(ADMIN_EMAIL, ADMIN_PASSWORD).await;
    let code = app.latest_code(ADMIN_EMAIL).await;

    // Simulate 5+ minutes passing.
    sqlx::query("UPDATE login_challenges SET expires_at = now() - interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();

    let res = app.verify(&challenge, &code).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
    assert_eq!(res.body["error"]["code"], "CODE_EXPIRED");
}

#[sqlx::test(migrations = "./migrations")]
async fn reused_code_is_rejected(pool: PgPool) {
    let app = TestApp::new(pool);
    app.seed().await;

    let challenge = app.start_login(ADMIN_EMAIL, ADMIN_PASSWORD).await;
    let code = app.latest_code(ADMIN_EMAIL).await;

    assert_eq!(app.verify(&challenge, &code).await.status, StatusCode::OK);
    let again = app.verify(&challenge, &code).await;
    assert_eq!(again.status, StatusCode::UNAUTHORIZED);
    assert_eq!(again.body["error"]["code"], "CODE_ALREADY_USED");
}

#[sqlx::test(migrations = "./migrations")]
async fn new_login_supersedes_previous_challenge(pool: PgPool) {
    let app = TestApp::new(pool);
    app.seed().await;

    let first = app.start_login(ADMIN_EMAIL, ADMIN_PASSWORD).await;
    let first_code = app.latest_code(ADMIN_EMAIL).await;
    let _second = app.start_login(ADMIN_EMAIL, ADMIN_PASSWORD).await;

    let res = app.verify(&first, &first_code).await;
    assert_eq!(res.status, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn malformed_requests_are_rejected_with_json_errors(pool: PgPool) {
    let app = TestApp::new(pool);
    app.seed().await;

    let bad_email = app
        .post(
            "/auth/login",
            None,
            json!({ "email": "not-an-email", "password": "x" }),
        )
        .await;
    assert_eq!(bad_email.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(bad_email.body["error"]["code"], "VALIDATION_ERROR");

    let bad_code = app
        .post(
            "/auth/verify-2fa",
            None,
            json!({ "login_challenge_id": uuid::Uuid::new_v4(), "code": "12ab56" }),
        )
        .await;
    assert_eq!(bad_code.status, StatusCode::UNPROCESSABLE_ENTITY);

    let unknown_challenge = app
        .verify(&uuid::Uuid::new_v4().to_string(), "123456")
        .await;
    assert_eq!(unknown_challenge.status, StatusCode::UNAUTHORIZED);
    assert_eq!(unknown_challenge.body["error"]["code"], "INVALID_CHALLENGE");
}

#[sqlx::test(migrations = "./migrations")]
async fn protected_routes_require_a_valid_token(pool: PgPool) {
    let app = TestApp::new(pool);

    let missing = app.get("/tasks/view-my-tasks", None).await;
    assert_eq!(missing.status, StatusCode::UNAUTHORIZED);
    assert_eq!(missing.body["error"]["code"], "UNAUTHORIZED");

    let garbage = app.get("/tasks/view-my-tasks", Some("not.a.jwt")).await;
    assert_eq!(garbage.status, StatusCode::UNAUTHORIZED);
}
