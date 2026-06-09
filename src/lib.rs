use axum::{routing::{get, post}, Router};
use std::sync::Arc;
use crate::handlers::AppState;
use crate::handlers::tasks::*;

pub mod models;
pub mod repository;
pub mod handlers;

pub fn build_app(shared_state: Arc<AppState>) -> Router {
    Router::new()
        .route("/seed/users", post(seed_users))
        .route("/auth/login", post(login))
        .route("/dev/email-logs/latest", get(get_latest_email))
        .route("/auth/verify-2fa", post(verify_2fa))
        .route("/tasks", post(create_task))
        .route("/tasks/assign", post(assign_tasks))
        .route("/tasks/view-my-tasks", get(view_my_tasks))
        .with_state(shared_state)
}
