//! Test harness: builds the real router in-process (no network) on top of an
//! isolated database created per test by `#[sqlx::test]`.

#![allow(dead_code)]

use std::sync::Arc;

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sqlx::PgPool;
use task_manager_api::{
    build_router,
    config::Config,
    services::cache::{Cache, InMemoryCache},
    AppState,
};
use tower::ServiceExt;

pub const ADMIN_EMAIL: &str = "admin@example.com";
pub const ADMIN_PASSWORD: &str = "Admin@12345";
pub const BOND_EMAIL: &str = "jamesbond@example.com";
pub const BOND_PASSWORD: &str = "Bond@007007";

pub struct TestApp {
    pub router: Router,
    pub pool: PgPool,
}

pub struct TestResponse {
    pub status: StatusCode,
    pub body: Value,
}

impl TestApp {
    pub fn new(pool: PgPool) -> Self {
        Self::with_cache(pool, Arc::new(InMemoryCache::default()))
    }

    pub fn with_cache(pool: PgPool, cache: Arc<dyn Cache>) -> Self {
        let state = AppState::new(pool.clone(), Config::for_tests("unused"), cache);
        Self {
            router: build_router(state),
            pool,
        }
    }

    pub async fn request(
        &self,
        method: Method,
        path: &str,
        token: Option<&str>,
        body: Option<Value>,
    ) -> TestResponse {
        let mut builder = Request::builder().method(method).uri(path);
        if let Some(t) = token {
            builder = builder.header(header::AUTHORIZATION, format!("Bearer {t}"));
        }
        let req = match body {
            Some(b) => builder
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(b.to_string())),
            None => builder.body(Body::empty()),
        }
        .unwrap();

        let res = self.router.clone().oneshot(req).await.unwrap();
        let status = res.status();
        let bytes = res.into_body().collect().await.unwrap().to_bytes();
        let body = if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes)
                .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into()))
        };
        TestResponse { status, body }
    }

    pub async fn get(&self, path: &str, token: Option<&str>) -> TestResponse {
        self.request(Method::GET, path, token, None).await
    }

    pub async fn post(&self, path: &str, token: Option<&str>, body: Value) -> TestResponse {
        self.request(Method::POST, path, token, Some(body)).await
    }

    pub async fn patch(&self, path: &str, token: Option<&str>, body: Value) -> TestResponse {
        self.request(Method::PATCH, path, token, Some(body)).await
    }

    pub async fn seed(&self) {
        let res = self.request(Method::POST, "/seed/users", None, None).await;
        assert_eq!(res.status, StatusCode::OK, "{:?}", res.body);
    }

    /// Step 1 of login. Returns the challenge id.
    pub async fn start_login(&self, email: &str, password: &str) -> String {
        let res = self
            .post(
                "/auth/login",
                None,
                json!({ "email": email, "password": password }),
            )
            .await;
        assert_eq!(res.status, StatusCode::OK, "{:?}", res.body);
        res.body["login_challenge_id"].as_str().unwrap().to_string()
    }

    /// Reads the latest code from the dev email log.
    pub async fn latest_code(&self, email: &str) -> String {
        let res = self
            .get(&format!("/dev/email-logs/latest?email={email}"), None)
            .await;
        assert_eq!(res.status, StatusCode::OK, "{:?}", res.body);
        res.body["verification_code"].as_str().unwrap().to_string()
    }

    pub async fn verify(&self, challenge_id: &str, code: &str) -> TestResponse {
        self.post(
            "/auth/verify-2fa",
            None,
            json!({ "login_challenge_id": challenge_id, "code": code }),
        )
        .await
    }

    /// Full login + 2FA; returns the JWT.
    pub async fn login(&self, email: &str, password: &str) -> String {
        let challenge = self.start_login(email, password).await;
        let code = self.latest_code(email).await;
        let res = self.verify(&challenge, &code).await;
        assert_eq!(res.status, StatusCode::OK, "{:?}", res.body);
        res.body["access_token"].as_str().unwrap().to_string()
    }

    pub async fn create_task(&self, token: &str, title: &str, priority: &str) -> TestResponse {
        self.post(
            "/tasks",
            Some(token),
            json!({ "title": title, "description": format!("{title} details"), "priority": priority }),
        )
        .await
    }
}

/// A code guaranteed to differ from `code`.
pub fn wrong_code(code: &str) -> String {
    let n: u32 = code.parse().unwrap();
    format!("{:06}", (n + 1) % 1_000_000)
}
