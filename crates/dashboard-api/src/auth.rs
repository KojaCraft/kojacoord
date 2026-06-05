use axum::{async_trait, extract::FromRequestParts, http::request::Parts};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{error::AppError, routes::AppState};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: u64,
}

pub fn create_token(
    username: &str,
    role: &str,
    secret: &str,
    expiry_hours: u64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let exp =
        (chrono::Utc::now() + chrono::Duration::hours(expiry_hours as i64)).timestamp() as u64;
    let claims = Claims {
        sub: username.into(),
        role: role.into(),
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}

pub struct AuthUser(pub Claims);

#[async_trait]
impl FromRequestParts<Arc<AppState>> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<AppState>,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .ok_or(AppError::Unauthorized)?;

        let claims =
            verify_token(token, &state.cfg.jwt.secret).map_err(|_| AppError::Unauthorized)?;
        Ok(AuthUser(claims))
    }
}

pub fn require_admin(claims: &Claims) -> Result<(), AppError> {
    if matches!(claims.role.as_str(), "ADMIN" | "SUPERADMIN") {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

pub fn require_superadmin(claims: &Claims) -> Result<(), AppError> {
    if claims.role == "SUPERADMIN" {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

pub fn require_moderator(claims: &Claims) -> Result<(), AppError> {
    if matches!(claims.role.as_str(), "MODERATOR" | "ADMIN" | "SUPERADMIN") {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}
