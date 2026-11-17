mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{json, Value};
use sqlx::PgPool;

/// Admin creates 5 tasks and assigns the first 3 to James Bond.
/// Returns (admin_token, bond_token, all task ids).
async fn setup_assignment(app: &TestApp) -> (String, String, Vec<String>) {
    app.seed().await;
    let admin = app.login(ADMIN_EMAIL, ADMIN_PASSWORD).await;

    let specs = [
        ("Infiltrate SPECTRE HQ", "high"),
        ("Recover the Lektor decoder", "medium"),
        ("Brief M on Operation Thunderball", "low"),
        ("Audit Q-Branch gadget inventory", "medium"),
        ("Renew Aston Martin insurance", "low"),
    ];
    let mut ids = Vec::new();
    for (title, priority) in specs {
        let res = app.create_task(&admin, title, priority).await;
        assert_eq!(res.status, StatusCode::CREATED, "{:?}", res.body);
        assert_eq!(res.body["status"], "todo");
        ids.push(res.body["id"].as_str().unwrap().to_string());
    }

    let assign = app
        .post(
            "/tasks/assign",
            Some(&admin),
            json!({ "task_ids": &ids[..3], "assignee_email": BOND_EMAIL }),
        )
        .await;
    assert_eq!(assign.status, StatusCode::OK, "{:?}", assign.body);
    assert_eq!(assign.body["assigned_count"], 3);

    let bond = app.login(BOND_EMAIL, BOND_PASSWORD).await;
    (admin, bond, ids)
}

#[sqlx::test(migrations = "./migrations")]
async fn full_validation_workflow(pool: PgPool) {
    let app = TestApp::new(pool.clone());
    let (admin, bond, _ids) = setup_assignment(&app).await;

    // Exactly 5 tasks exist, exactly 3 belong to James Bond.
    let all = app.get("/tasks", Some(&admin)).await;
    assert_eq!(all.body.as_array().unwrap().len(), 5);
    let bond_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM tasks t JOIN users u ON u.id = t.assigned_to_id WHERE u.email = $1",
    )
    .bind(BOND_EMAIL)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(bond_count, 3);

    // James Bond cannot create tasks.
    let forbidden = app.create_task(&bond, "Rogue task", "high").await;
    assert_eq!(forbidden.status, StatusCode::FORBIDDEN);
    assert_eq!(forbidden.body["error"]["code"], "FORBIDDEN");

    // First call: from the database.
    let first = app.get("/tasks/view-my-tasks", Some(&bond)).await;
    assert_eq!(first.status, StatusCode::OK);
    assert_eq!(
        first.body["user"],
        json!({ "email": BOND_EMAIL, "role": "staff" })
    );
    assert_eq!(first.body["summary"]["total_assigned_tasks"], 3);
    assert_eq!(first.body["cache"]["hit"], false);
    let tasks = first.body["tasks"].as_array().unwrap();
    assert_eq!(tasks.len(), 3);
    let priorities: Vec<&str> = tasks
        .iter()
        .map(|t| t["priority"].as_str().unwrap())
        .collect();
    assert_eq!(priorities, ["high", "medium", "low"]);
    assert!(tasks
        .iter()
        .all(|t| t["assigned_to"] == BOND_EMAIL && t["status"] == "todo"));

    // Second call: from the cache, identical data.
    let second = app.get("/tasks/view-my-tasks", Some(&bond)).await;
    assert_eq!(second.body["cache"]["hit"], true);
    assert_eq!(second.body["tasks"], first.body["tasks"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn staff_cannot_use_admin_endpoints(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_admin, bond, ids) = setup_assignment(&app).await;

    let assign = app
        .post(
            "/tasks/assign",
            Some(&bond),
            json!({ "task_ids": [&ids[3]], "assignee_email": BOND_EMAIL }),
        )
        .await;
    assert_eq!(assign.status, StatusCode::FORBIDDEN);
    assert_eq!(
        app.get("/tasks", Some(&bond)).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.get("/users", Some(&bond)).await.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.patch(
            &format!("/tasks/{}", ids[0]),
            Some(&bond),
            json!({ "status": "done" })
        )
        .await
        .status,
        StatusCode::FORBIDDEN
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn admin_sees_only_own_assignments_in_view_my_tasks(pool: PgPool) {
    let app = TestApp::new(pool);
    let (admin, _bond, _ids) = setup_assignment(&app).await;

    let mine = app.get("/tasks/view-my-tasks", Some(&admin)).await;
    assert_eq!(mine.status, StatusCode::OK);
    assert_eq!(mine.body["summary"]["total_assigned_tasks"], 0);
    assert_eq!(mine.body["tasks"], json!([]));
}

#[sqlx::test(migrations = "./migrations")]
async fn assignment_invalidates_the_assignee_cache(pool: PgPool) {
    let app = TestApp::new(pool);
    let (admin, bond, ids) = setup_assignment(&app).await;

    assert_eq!(
        app.get("/tasks/view-my-tasks", Some(&bond)).await.body["cache"]["hit"],
        false
    );
    assert_eq!(
        app.get("/tasks/view-my-tasks", Some(&bond)).await.body["cache"]["hit"],
        true
    );

    let res = app
        .post(
            "/tasks/assign",
            Some(&admin),
            json!({ "task_ids": [&ids[3]], "assignee_email": BOND_EMAIL }),
        )
        .await;
    assert_eq!(res.status, StatusCode::OK);

    let after = app.get("/tasks/view-my-tasks", Some(&bond)).await;
    assert_eq!(
        after.body["cache"]["hit"], false,
        "cache must be invalidated after assignment"
    );
    assert_eq!(after.body["summary"]["total_assigned_tasks"], 4);
}

#[sqlx::test(migrations = "./migrations")]
async fn update_invalidates_the_assignee_cache(pool: PgPool) {
    let app = TestApp::new(pool);
    let (admin, bond, ids) = setup_assignment(&app).await;

    app.get("/tasks/view-my-tasks", Some(&bond)).await;
    assert_eq!(
        app.get("/tasks/view-my-tasks", Some(&bond)).await.body["cache"]["hit"],
        true
    );

    let upd = app
        .patch(
            &format!("/tasks/{}", ids[0]),
            Some(&admin),
            json!({ "status": "in_progress" }),
        )
        .await;
    assert_eq!(upd.status, StatusCode::OK, "{:?}", upd.body);
    assert_eq!(upd.body["status"], "in_progress");

    let after = app.get("/tasks/view-my-tasks", Some(&bond)).await;
    assert_eq!(after.body["cache"]["hit"], false);
    let statuses: Vec<&Value> = after.body["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| &t["status"])
        .collect();
    assert!(statuses.contains(&&json!("in_progress")));
}

#[sqlx::test(migrations = "./migrations")]
async fn assign_validates_tasks_and_assignee(pool: PgPool) {
    let app = TestApp::new(pool);
    let (admin, _bond, ids) = setup_assignment(&app).await;

    let missing_task = app
        .post(
            "/tasks/assign",
            Some(&admin),
            json!({ "task_ids": [uuid::Uuid::new_v4()], "assignee_email": BOND_EMAIL }),
        )
        .await;
    assert_eq!(missing_task.status, StatusCode::NOT_FOUND);

    let unknown_user = app
        .post(
            "/tasks/assign",
            Some(&admin),
            json!({ "task_ids": [&ids[3]], "assignee_email": "m@example.com" }),
        )
        .await;
    assert_eq!(unknown_user.status, StatusCode::NOT_FOUND);

    let to_admin = app
        .post(
            "/tasks/assign",
            Some(&admin),
            json!({ "task_ids": [&ids[3]], "assignee_email": ADMIN_EMAIL }),
        )
        .await;
    assert_eq!(to_admin.status, StatusCode::UNPROCESSABLE_ENTITY);

    let empty = app
        .post(
            "/tasks/assign",
            Some(&admin),
            json!({ "task_ids": [], "assignee_email": BOND_EMAIL }),
        )
        .await;
    assert_eq!(empty.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[sqlx::test(migrations = "./migrations")]
async fn create_task_validates_input(pool: PgPool) {
    let app = TestApp::new(pool);
    app.seed().await;
    let admin = app.login(ADMIN_EMAIL, ADMIN_PASSWORD).await;

    let blank = app
        .post("/tasks", Some(&admin), json!({ "title": "   " }))
        .await;
    assert_eq!(blank.status, StatusCode::UNPROCESSABLE_ENTITY);

    let bad_priority = app
        .post(
            "/tasks",
            Some(&admin),
            json!({ "title": "x", "priority": "urgent" }),
        )
        .await;
    assert_eq!(bad_priority.status, StatusCode::UNPROCESSABLE_ENTITY);

    let defaults = app
        .post("/tasks", Some(&admin), json!({ "title": "Defaults" }))
        .await;
    assert_eq!(defaults.status, StatusCode::CREATED);
    assert_eq!(defaults.body["status"], "todo");
    assert_eq!(defaults.body["priority"], "medium");
    assert_eq!(defaults.body["created_by"], ADMIN_EMAIL);
    assert_eq!(defaults.body["assigned_to"], Value::Null);
}

#[sqlx::test(migrations = "./migrations")]
async fn dev_reset_clears_tasks_and_cache(pool: PgPool) {
    let app = TestApp::new(pool);
    let (_admin, bond, _ids) = setup_assignment(&app).await;
    app.get("/tasks/view-my-tasks", Some(&bond)).await;

    let res = app.post("/dev/reset", None, json!({})).await;
    assert_eq!(res.status, StatusCode::OK);
    assert_eq!(res.body["deleted_tasks"], 5);

    let after = app.get("/tasks/view-my-tasks", Some(&bond)).await;
    assert_eq!(after.body["cache"]["hit"], false);
    assert_eq!(after.body["summary"]["total_assigned_tasks"], 0);
}
