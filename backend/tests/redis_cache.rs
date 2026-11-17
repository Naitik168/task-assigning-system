//! Exercises the Redis cache backend. Skipped unless `TEST_REDIS_URL` is set,
//! e.g. `TEST_REDIS_URL=redis://localhost:6379 cargo test`.

mod common;

use std::{sync::Arc, time::Duration};

use axum::http::StatusCode;
use common::*;
use serde_json::json;
use sqlx::PgPool;
use task_manager_api::services::cache::{Cache, RedisCache};

async fn redis() -> Option<RedisCache> {
    let url = std::env::var("TEST_REDIS_URL").ok()?;
    Some(
        RedisCache::connect(&url)
            .await
            .expect("connect to TEST_REDIS_URL"),
    )
}

#[tokio::test]
async fn redis_set_get_delete() {
    let Some(cache) = redis().await else {
        eprintln!("skipping: TEST_REDIS_URL not set");
        return;
    };
    let key = format!("taskmgr:test:{}", uuid::Uuid::new_v4());
    cache
        .set(&key, "value".into(), Duration::from_secs(30))
        .await
        .unwrap();
    assert_eq!(cache.get(&key).await.unwrap().as_deref(), Some("value"));
    cache.delete(std::slice::from_ref(&key)).await.unwrap();
    assert_eq!(cache.get(&key).await.unwrap(), None);
}

#[sqlx::test(migrations = "./migrations")]
async fn view_my_tasks_hits_redis_and_assignment_invalidates(pool: PgPool) {
    let Some(cache) = redis().await else {
        eprintln!("skipping: TEST_REDIS_URL not set");
        return;
    };
    let app = TestApp::with_cache(pool, Arc::new(cache));
    app.seed().await;
    let admin = app.login(ADMIN_EMAIL, ADMIN_PASSWORD).await;
    let bond = app.login(BOND_EMAIL, BOND_PASSWORD).await;

    let task = app.create_task(&admin, "Redis-backed task", "high").await;
    let id = task.body["id"].as_str().unwrap();
    app.post(
        "/tasks/assign",
        Some(&admin),
        json!({ "task_ids": [id], "assignee_email": BOND_EMAIL }),
    )
    .await;

    let first = app.get("/tasks/view-my-tasks", Some(&bond)).await;
    assert_eq!(first.status, StatusCode::OK);
    assert_eq!(first.body["cache"]["hit"], false);
    assert_eq!(
        app.get("/tasks/view-my-tasks", Some(&bond)).await.body["cache"]["hit"],
        true
    );

    app.patch(
        &format!("/tasks/{id}"),
        Some(&admin),
        json!({ "status": "done" }),
    )
    .await;
    assert_eq!(
        app.get("/tasks/view-my-tasks", Some(&bond)).await.body["cache"]["hit"],
        false
    );
}
