// Replace the top section of src/handlers/tasks.rs with this exact code:
use axum::{
    extract::State,
    Json,
    http::StatusCode,
    debug_handler
};
use jsonwebtoken::{encode, EncodingKey, Header};
use redis::AsyncCommands;
use chrono::Utc;
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2, PasswordVerifier, PasswordHash
};
use rand::RngExt; 

use crate::models::*;
use crate::repository::{user_repo::UserRepository, task_repo::TaskRepository};
use crate::handlers::{SharedState, AuthUser};

/// helper utility to hash passwords/codes securely via Argon2id
fn hash_secret(secret: &str) -> String {
    let mut salt_bytes = [0u8; 16];
    
    rand::rng().fill(&mut salt_bytes); 
    
    let salt = SaltString::encode_b64(&salt_bytes).unwrap();
    let argon2 = Argon2::default();
    argon2.hash_password(secret.as_bytes(), &salt).unwrap().to_string()
}

/// helper utility to verify Argon2 hashes
fn verify_hash(secret: &str, hash: &str) -> bool {
    if let Ok(parsed_hash) = PasswordHash::new(hash) {
        Argon2::default().verify_password(secret.as_bytes(), &parsed_hash).is_ok()
    } else {
        false
    }
}

// 1. POST /seed/users
pub async fn seed_users(State(state): State<SharedState>) -> Result<Json<SeedUsersResponse>, (StatusCode, String)> {
    let admin_email = "admin@company.com";
    let jb_email = "jamesbond@example.com";

    // Clean check to prevent duplicate seeding crash
    if let Ok(Some(_)) = UserRepository::find_by_email(&state.db, admin_email).await {
        return Err((StatusCode::BAD_REQUEST, "Database already seeded".to_string()));
    }

    let admin_hash = hash_secret("AdminPassword123");
    let jb_hash = hash_secret("ShakenNotStirred");

    let admin = UserRepository::create_user(&state.db, "Admin User", admin_email, &admin_hash, UserRole::Admin)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let jb = UserRepository::create_user(&state.db, "James Bond", jb_email, &jb_hash, UserRole::Staff)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(SeedUsersResponse {
        admin_id: admin.id,
        james_bond_id: jb.id,
        message: "Admin and James Bond users successfully created.".to_string(),
    }))
}

// 2. POST /auth/login
pub async fn login(
    State(state): State<SharedState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    let user = UserRepository::find_by_email(&state.db, &payload.email)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::UNAUTHORIZED, "Invalid email or password".to_string()))?;

    if !verify_hash(&payload.password, &user.hashed_password) {
        return Err((StatusCode::UNAUTHORIZED, "Invalid email or password".to_string()));
    }

// Inside pub async fn login (...) look for plain_code generation:
let numeric_code: u32 = rand::rng().random_range(100000..1000000);
let plain_code = format!("{:06}", numeric_code);
let hashed_code = hash_secret(&plain_code);
    // Persist code through repo (hashes code in main table, logs plain code to dev_email_logs)
    let challenge_id = UserRepository::create_2fa_challenge(
        &state.db, 
        user.id, 
        &user.email, 
        &plain_code, 
        &hashed_code
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Output code to console output as fallback per requirement rules
    println!("[2FA EMAIL SIMULATOR] To: {}, Code: {}, Challenge: {}", user.email, plain_code, challenge_id);

    Ok(Json(LoginResponse {
        login_challenge_id: challenge_id,
        message: "Two-factor code sent to email logs.".to_string(),
    }))
}

// 3. GET /dev/email-logs/latest
pub async fn get_latest_email(State(state): State<SharedState>) -> Result<Json<LatestEmailResponse>, (StatusCode, String)> {
    let log = UserRepository::get_latest_email_log(&state.db)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "No email logs found".to_string()))?;

    Ok(Json(LatestEmailResponse {
        email: log.email,
        code: log.plain_code,
        login_challenge_id: log.login_challenge_id,
    }))
}

// 4. POST /auth/verify-2fa
pub async fn verify_2fa(
    State(state): State<SharedState>,
    Json(payload): Json<Verify2FaRequest>,
) -> Result<Json<Verify2FaResponse>, (StatusCode, String)> {
    // Transactionally reads and burns the row if current time is within bounds
    let challenge = UserRepository::verify_and_consume_challenge(&state.db, payload.login_challenge_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::BAD_REQUEST, "Invalid, expired, or already used 2FA code".to_string()))?;

    if !verify_hash(&payload.code, &challenge.hashed_code) {
        return Err((StatusCode::BAD_REQUEST, "Invalid, expired, or already used 2FA code".to_string()));
    }

    let user = UserRepository::find_by_id(&state.db, challenge.user_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "User missing".to_string()))?;

    // Issue JWT Access Token
    let expiration = Utc::now() + chrono::Duration::hours(2);
    let claims = Claims {
        sub: user.id,
        email: user.email,
        role: user.role,
        exp: expiration.timestamp(),
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(Verify2FaResponse {
        token,
        token_type: "Bearer".to_string(),
    }))
}

// 5. POST /tasks (Admin Only)
#[debug_handler]
pub async fn create_task(
    AuthUser(claims): AuthUser,
    State(state): State<SharedState>,
    Json(payload): Json<CreateTaskRequest>,
) -> Result<(StatusCode, String), (StatusCode, String)> {
    // Assert Admin Role Requirement
    if claims.role != UserRole::Admin {
        return Err((StatusCode::FORBIDDEN, "Forbidden: Only administrators can create tasks".to_string()));
    }

    TaskRepository::create_task(
        &state.db,
        &payload.title,
        payload.description,
        payload.priority,
        claims.sub
    )
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok((StatusCode::CREATED, "Task successfully created".to_string()))
}

// 6. POST /tasks/assign (Admin Only + Cache Invalidation)
#[debug_handler]
pub async fn assign_tasks(
    AuthUser(claims): AuthUser,
    State(state): State<SharedState>,
    Json(payload): Json<AssignTasksRequest>,
) -> Result<Json<AssignTasksResponse>, (StatusCode, String)> {
    if claims.role != UserRole::Admin {
        return Err((StatusCode::FORBIDDEN, "Forbidden: Only administrators can assign tasks".to_string()));
    }

    TaskRepository::assign_tasks_to_user(&state.db, payload.user_id, &payload.task_ids)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // --- CRITICAL CACHE INVALIDATION LAYER ---
    // Instantly invalidate the specific assigned user's cached timeline representation
    let cache_key = format!("cache:tasks:{}", payload.user_id);
    let mut redis_conn = state.redis.get_multiplexed_async_connection().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    
    let _: () = redis_conn.del(cache_key).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(AssignTasksResponse {
        message: format!("Successfully assigned {} tasks and cleared target cache.", payload.task_ids.len()),
    }))
}

// 7. GET /tasks/view-my-tasks (Caching Flow Implementation)
#[debug_handler]
pub async fn view_my_tasks(
    AuthUser(claims): AuthUser,
    State(state): State<SharedState>,
) -> Result<Json<ViewTasksResponse>, (StatusCode, String)> {
    let cache_key = format!("cache:tasks:{}", claims.sub);
    
    let mut redis_conn = state.redis.get_multiplexed_async_connection().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Attempt Redis Fetch
    let cached_data: Option<String> = redis_conn.get(&cache_key).await.ok();

    if let Some(json_str) = cached_data {
        // Parse and modify metadata context layer to true
        if let Ok(mut response) = serde_json::from_str::<ViewTasksResponse>(&json_str) {
            response.cache.hit = true;
            return Ok(Json(response));
        }
    }

    // Fallback cache miss -> DB Query
    let tasks_list = TaskRepository::get_tasks_for_user(&state.db, claims.sub)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let response_payload = ViewTasksResponse {
        user: UserSummaryDto {
            email: claims.email,
            role: claims.role,
        },
        tasks: tasks_list.clone(),
        summary: SummaryDto {
            total_assigned_tasks: tasks_list.len() as i64,
        },
        cache: CacheMetadata { hit: false },
    };

    // Serialize and cache raw payload back to Redis (TTL 1 Hour)
    if let Ok(serialized) = serde_json::to_string(&response_payload) {
        let _: () = redis_conn.set_ex(&cache_key, serialized, 3600).await
            .unwrap_or_default();
    }

    Ok(Json(response_payload))
}
