pub mod tasks;

use axum::{
    extract::FromRequestParts,
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

impl FromRequestParts<SharedState> for AuthUser {
    type Rejection = (StatusCode, String);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &SharedState,
    ) -> Result<Self, Self::Rejection> {

        let auth_header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or((
                StatusCode::UNAUTHORIZED,
                "Missing Authorization header".to_string(),
            ))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or((
                StatusCode::UNAUTHORIZED,
                "Invalid token format".to_string(),
            ))?;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.jwt_secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| (
            StatusCode::UNAUTHORIZED,
            "Invalid token".to_string(),
        ))?;

        Ok(AuthUser(token_data.claims))
    }
}
