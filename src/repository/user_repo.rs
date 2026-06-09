use sqlx::PgPool;
use uuid::Uuid;
use chrono::{Utc, Duration};
use crate::models::{User, UserRole, LoginChallenge, DevEmailLog};

pub struct UserRepository;

impl UserRepository {
    pub async fn create_user(
        pool: &PgPool, 
        full_name: &str,
        email: &str, 
        hashed_password: &str, 
        role: UserRole
    ) -> Result<User, sqlx::Error> {
        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (full_name, email, hashed_password, role) VALUES ($1, $2, $3, $4) 
             RETURNING id, full_name, email, hashed_password, role, created_at, updated_at"
        )
        .bind(full_name)
        .bind(email)
        .bind(hashed_password)
        .bind(role)
        .fetch_one(pool)
        .await?;
        
        Ok(user)
    }

    pub async fn find_by_email(pool: &PgPool, email: &str) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE email = $1")
            .bind(email)
            .fetch_optional(pool)
            .await
    }

    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<User>, sqlx::Error> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(id)
            .fetch_optional(pool)
            .await
    }

    // Handles strict 2FA generation requirements (Hashed in DB, plain in Dev Logs)
    pub async fn create_2fa_challenge(
        pool: &PgPool,
        user_id: Uuid,
        email: &str,
        plain_code: &str,
        hashed_code: &str,
    ) -> Result<Uuid, sqlx::Error> {
        let mut tx = pool.begin().await?;

        let expires_at = Utc::now() + Duration::minutes(5);

        // 1. Insert into real challenge tracker
        let challenge_id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO login_challenges (user_id, hashed_code, expires_at) 
             VALUES ($1, $2, $3) RETURNING id"
        )
        .bind(user_id)
        .bind(hashed_code)
        .bind(expires_at)
        .fetch_one(&mut *tx)
        .await?;

        // 2. Insert into Dev logs back door for retrieval
        sqlx::query(
            "INSERT INTO dev_email_logs (email, plain_code, login_challenge_id) 
             VALUES ($1, $2, $3)"
        )
        .bind(email)
        .bind(plain_code)
        .bind(challenge_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(challenge_id)
    }

    // Dev backdoor reader
    pub async fn get_latest_email_log(pool: &PgPool) -> Result<Option<DevEmailLog>, sqlx::Error> {
        sqlx::query_as::<_, DevEmailLog>(
            "SELECT * FROM dev_email_logs ORDER BY created_at DESC LIMIT 1"
        )
        .fetch_optional(pool)
        .await
    }

    // Verify 2FA challenge with atomicity
    pub async fn verify_and_consume_challenge(
        pool: &PgPool,
        challenge_id: Uuid,
    ) -> Result<Option<LoginChallenge>, sqlx::Error> {
        let mut tx = pool.begin().await?;

        // Look up valid, unexpired, unused challenge rows
        let challenge = sqlx::query_as::<_, LoginChallenge>(
            "SELECT * FROM login_challenges 
             WHERE id = $1 AND is_used = FALSE AND expires_at > NOW()"
        )
        .bind(challenge_id)
        .fetch_optional(&mut *tx)
        .await?;

        if challenge.is_some() {
            // Enforce single-use restriction instantly
            sqlx::query("UPDATE login_challenges SET is_used = TRUE WHERE id = $1")
                .bind(challenge_id)
                .execute(&mut *tx)
                .await?;
        }

        tx.commit().await?;
        Ok(challenge)
    }
}
