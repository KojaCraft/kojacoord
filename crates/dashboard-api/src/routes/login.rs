use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{auth, db, error::AppError, routes::AppState};

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,

    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,

    pub role: String,
}

pub async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, AppError> {
    let user = db::find_admin(&state.pool, &req.username)
        .await?
        .ok_or(AppError::Unauthorized)?;

    let valid = bcrypt::verify(&req.password, &user.password_hash)
        .map_err(|e| AppError::Internal(e.to_string()))?;
    if !valid {
        return Err(AppError::Unauthorized);
    }

    db::update_last_login(&state.pool, user.id).await?;

    let token = auth::create_token(
        &user.username,
        &user.role,
        &state.cfg.jwt.secret,
        state.cfg.jwt.expiry_hours,
    )
    .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(LoginResponse {
        token,
        role: user.role,
    }))
}
