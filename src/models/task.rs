use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "user_role", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Admin,
    Staff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "task_priority", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type, Serialize, Deserialize)]
#[sqlx(type_name = "task_status", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct LoginChallenge {
    pub id: Uuid,
    pub user_id: Uuid,
    pub hashed_code: String,
    pub expires_at: DateTime<Utc>,
    pub is_used: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub created_by: Uuid,
    pub assigned_to_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct DevEmailLog {
    pub id: Uuid,
    pub email: String,
    pub plain_code: String,
    pub login_challenge_id: Uuid,
    pub created_at: DateTime<Utc>,
}

// POST /auth/login
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub login_challenge_id: Uuid,
    pub message: String,
}

// POST /auth/verify-2fa
#[derive(Debug, Deserialize)]
pub struct Verify2FaRequest {
    pub login_challenge_id: Uuid,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct Verify2FaResponse {
    pub token: String,
    pub token_type: String, // E.g., "Bearer"
}

// GET /dev/email-logs/latest
#[derive(Debug, Serialize)]
pub struct LatestEmailResponse {
    pub email: String,
    pub code: String,
    pub login_challenge_id: Uuid,
}

// GET /tasks/view-my-tasks (Response Models)

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserSummaryDto {
    pub email: String,
    pub role: UserRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TaskDto {
    pub id: Uuid,
    pub title: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub assigned_to: String, // Populated via a table join to user email
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryDto {
    pub total_assigned_tasks: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetadata {
    pub hit: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct AssignTasksResponse {
    pub message: String,
}

// 4. Ensure CreateTaskRequest and AssignTasksRequest are present
#[derive(Debug, serde::Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub description: Option<String>,
    pub priority: crate::models::TaskPriority,
}

#[derive(Debug, serde::Deserialize)]
pub struct AssignTasksRequest {
    pub user_id: uuid::Uuid,
    pub task_ids: Vec<uuid::Uuid>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct SeedUsersResponse {
    pub admin_id: uuid::Uuid,
    pub james_bond_id: uuid::Uuid,
    pub message: String,
}

// Parent Wrapper Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewTasksResponse {
    pub user: UserSummaryDto,
    pub tasks: Vec<TaskDto>,
    pub summary: SummaryDto,
    pub cache: CacheMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid, // User ID
    pub email: String,
    pub role: UserRole,
    pub exp: i64,
}
