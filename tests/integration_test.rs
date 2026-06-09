use std::sync::Arc;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use reqwest::Client;
use serde_json::Value;
use assesment_rust::{build_app, handlers::AppState};

#[tokio::test]
async fn test_full_workflow() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Setup Postgres & Redis connection
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/assessment_db".to_string());
    
    let pg_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Run database migrations in the test context
    sqlx::migrate!("./migrations")
        .run(&pg_pool)
        .await?;

    // Clean tables before starting tests to ensure a clean state
    sqlx::query("TRUNCATE TABLE dev_email_logs CASCADE").execute(&pg_pool).await?;
    sqlx::query("TRUNCATE TABLE tasks CASCADE").execute(&pg_pool).await?;
    sqlx::query("TRUNCATE TABLE login_challenges CASCADE").execute(&pg_pool).await?;
    sqlx::query("TRUNCATE TABLE users CASCADE").execute(&pg_pool).await?;

    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let redis_client = redis::Client::open(redis_url)?;

    // Flush Redis cache for a clean test run
    let mut redis_conn = redis_client.get_multiplexed_async_connection().await?;
    let _: () = redis::cmd("FLUSHALL").query_async(&mut redis_conn).await?;

    // 2. Build AppState and router
    let shared_state = Arc::new(AppState {
        db: pg_pool.clone(),
        redis: redis_client,
        jwt_secret: "TEST_JWT_SECRET_KEY_12345_DONOTUSEINPRODUCTION".to_string(),
    });

    let app = build_app(shared_state);

    // 3. Bind to a random port on localhost
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    
    // Spawn server in the background
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    let client = Client::new();
    let base_url = format!("http://{}", addr);

    // ==========================================
    // 1. Create two users: Admin and James Bond (POST /seed/users)
    // ==========================================
    let response = client
        .post(format!("{}/seed/users", base_url))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    
    let seed_data: Value = response.json().await?;
    let admin_id = seed_data["admin_id"].as_str().unwrap();
    let james_bond_id = seed_data["james_bond_id"].as_str().unwrap();

    assert!(!admin_id.is_empty());
    assert!(!james_bond_id.is_empty());

    // ==========================================
    // 2. Start login as Admin using email and password (POST /auth/login)
    // ==========================================
    let response = client
        .post(format!("{}/auth/login", base_url))
        .json(&serde_json::json!({
            "email": "admin@company.com",
            "password": "AdminPassword123"
        }))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    
    let login_data: Value = response.json().await?;
    let login_challenge_id = login_data["login_challenge_id"].as_str().unwrap();
    assert!(!login_challenge_id.is_empty());
    assert_eq!(login_data["message"], "Two-factor code sent to email logs.");

    // ==========================================
    // 3. Retrieve the verification code (GET /dev/email-logs/latest)
    // ==========================================
    let response = client
        .get(format!("{}/dev/email-logs/latest", base_url))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    
    let email_log_data: Value = response.json().await?;
    let admin_code = email_log_data["code"].as_str().unwrap();
    assert_eq!(email_log_data["email"], "admin@company.com");
    assert_eq!(email_log_data["login_challenge_id"], login_challenge_id);

    // ==========================================
    // 4. Verify Admin 2FA and receive an Admin JWT token (POST /auth/verify-2fa)
    // ==========================================
    let response = client
        .post(format!("{}/auth/verify-2fa", base_url))
        .json(&serde_json::json!({
            "login_challenge_id": login_challenge_id,
            "code": admin_code
        }))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    
    let verify_data: Value = response.json().await?;
    let admin_token = verify_data["token"].as_str().unwrap();
    assert_eq!(verify_data["token_type"], "Bearer");

    // Test that the code is single-use and cannot be used again
    let response_reuse = client
        .post(format!("{}/auth/verify-2fa", base_url))
        .json(&serde_json::json!({
            "login_challenge_id": login_challenge_id,
            "code": admin_code
        }))
        .send()
        .await?;
    assert_eq!(response_reuse.status(), reqwest::StatusCode::BAD_REQUEST);

    // ==========================================
    // 5. Create exactly 5 tasks as Admin (POST /tasks)
    // ==========================================
    let mut task_ids = Vec::new();
    let priorities = vec!["high", "medium", "low", "medium", "high"];
    
    for i in 1..=5 {
        let response = client
            .post(format!("{}/tasks", base_url))
            .header("Authorization", format!("Bearer {}", admin_token))
            .json(&serde_json::json!({
                "title": format!("Task {}", i),
                "description": format!("Description for task {}", i),
                "priority": priorities[i - 1]
            }))
            .send()
            .await?;
        assert_eq!(response.status(), reqwest::StatusCode::CREATED);
    }

    // Query DB directly to get the 5 task IDs
    let tasks_db: Vec<(uuid::Uuid,)> = sqlx::query_as("SELECT id FROM tasks")
        .fetch_all(&pg_pool)
        .await?;
    assert_eq!(tasks_db.len(), 5);
    for t in tasks_db {
        task_ids.push(t.0);
    }

    // ==========================================
    // 6. Assign exactly 3 of those tasks to James Bond (POST /tasks/assign)
    // ==========================================
    let assigned_task_ids = vec![task_ids[0], task_ids[1], task_ids[2]];
    let response = client
        .post(format!("{}/tasks/assign", base_url))
        .header("Authorization", format!("Bearer {}", admin_token))
        .json(&serde_json::json!({
            "user_id": james_bond_id,
            "task_ids": assigned_task_ids
        }))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    
    let assign_data: Value = response.json().await?;
    assert_eq!(assign_data["message"], "Successfully assigned 3 tasks and cleared target cache.");

    // ==========================================
    // 7. Start login as James Bond and retrieve his two-factor verification code
    // ==========================================
    let response = client
        .post(format!("{}/auth/login", base_url))
        .json(&serde_json::json!({
            "email": "jamesbond@example.com",
            "password": "ShakenNotStirred"
        }))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    
    let jb_login_data: Value = response.json().await?;
    let jb_challenge_id = jb_login_data["login_challenge_id"].as_str().unwrap();

    let response = client
        .get(format!("{}/dev/email-logs/latest", base_url))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    
    let jb_email_log: Value = response.json().await?;
    let jb_code = jb_email_log["code"].as_str().unwrap();
    assert_eq!(jb_email_log["email"], "jamesbond@example.com");

    // ==========================================
    // 8. Verify James Bond 2FA and receive a James Bond JWT token
    // ==========================================
    let response = client
        .post(format!("{}/auth/verify-2fa", base_url))
        .json(&serde_json::json!({
            "login_challenge_id": jb_challenge_id,
            "code": jb_code
        }))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    
    let jb_verify_data: Value = response.json().await?;
    let jb_token = jb_verify_data["token"].as_str().unwrap();

    // ==========================================
    // 9. Attempt to create a task as James Bond. This must return 403 Forbidden.
    // ==========================================
    let response = client
        .post(format!("{}/tasks", base_url))
        .header("Authorization", format!("Bearer {}", jb_token))
        .json(&serde_json::json!({
            "title": "JB Task",
            "description": "Should fail",
            "priority": "low"
        }))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::FORBIDDEN);

    // ==========================================
    // 10. Call GET /tasks/view-my-tasks as James Bond. It must return exactly 3 assigned tasks.
    // ==========================================
    let response = client
        .get(format!("{}/tasks/view-my-tasks", base_url))
        .header("Authorization", format!("Bearer {}", jb_token))
        .send()
        .await?;
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    
    let view_data: Value = response.json().await?;
    assert_eq!(view_data["user"]["email"], "jamesbond@example.com");
    assert_eq!(view_data["user"]["role"], "staff");
    assert_eq!(view_data["summary"]["total_assigned_tasks"], 3);
    assert_eq!(view_data["cache"]["hit"], false);
    
    let tasks_array = view_data["tasks"].as_array().unwrap();
    assert_eq!(tasks_array.len(), 3);
    for t in tasks_array {
        assert_eq!(t["assigned_to"], "jamesbond@example.com");
        assert_eq!(t["status"], "todo");
    }

    // ==========================================
    // 11. Call GET /tasks/view-my-tasks again. The response should come from cache and show cache.hit = true.
    // ==========================================
    let response_cache = client
        .get(format!("{}/tasks/view-my-tasks", base_url))
        .header("Authorization", format!("Bearer {}", jb_token))
        .send()
        .await?;
    assert_eq!(response_cache.status(), reqwest::StatusCode::OK);
    
    let view_data_cache: Value = response_cache.json().await?;
    assert_eq!(view_data_cache["cache"]["hit"], true);
    assert_eq!(view_data_cache["summary"]["total_assigned_tasks"], 3);

    Ok(())
}
