use axum::{routing::{get, post}, Router};
use std::sync::Arc;
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;

pub mod models;
pub mod repository;
pub mod handlers;

use crate::handlers::{AppState, tasks::*};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize structural tracing/logging (Optional but highly recommended for Axum)
    tracing_subscriber::fmt::init();

    // 2. Setup your Database connection pool (PostgreSQL)
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/assessment_db".to_string());
    
    let pg_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Run pending migrations automatically on startup
    sqlx::migrate!("./migrations")
        .run(&pg_pool)
        .await?;

    // 3. Setup your Cache connection client (Redis)
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let redis_client = redis::Client::open(redis_url)?;

    // 4. Wrap everything inside your custom Axum AppState container
    let shared_state = Arc::new(AppState {
        db: pg_pool,
        redis: redis_client,
        jwt_secret: std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "SUPER_SECRET_SIGNING_KEY_12345_DONOTUSEINPRODUCTION".to_string()),
    });

    // 5. Build up the API routing network matrix
    let app = Router::new()
        .route("/seed/users", post(seed_users))
        .route("/auth/login", post(login))
        .route("/dev/email-logs/latest", get(get_latest_email))
        .route("/auth/verify-2fa", post(verify_2fa))
        .route("/tasks", post(create_task))
        .route("/tasks/assign", post(assign_tasks))
        .route("/tasks/view-my-tasks", get(view_my_tasks))
        .with_state(shared_state);

    // 6. Bind the socket and ignite the hyper server using Tokio async loops
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    println!("🚀 Server securely humming along locally at http://{}", addr);
    axum::serve(listener, app).await?;

    Ok(())
}
