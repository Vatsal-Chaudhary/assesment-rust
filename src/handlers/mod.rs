pub mod tasks;

use axum::{
    extract::{FromRequestParts, State},
    http::{request::Parts, StatusCode},
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use sqlx::PgPool;
use std::sync::Arc;
use crate::models::*;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: redis::Client,
    pub jwt_secret: String,
}

pub type SharedState = Arc<AppState>;

pub struct AuthUser(pub Claims);

impl<S> FromRequestParts<S> for AuthUser
where
    SharedState: FromRequestParts<S>,
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        // Fix state extraction to cleanly resolve generic parameter types
        let shared_state = SharedState::from_request_parts(parts, state)
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "State extraction error".to_string()))?;

        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "Missing Authorization header".to_string()))?;

        if !auth_header.starts_with("Bearer ") {
            return Err((StatusCode::UNAUTHORIZED, "Invalid token type format".to_string()));
        }

        let token = &auth_header[7..];
        
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(shared_state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid or expired access token".to_string()))?;

        Ok(AuthUser(token_data.claims))
    }
}
