use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    auth::{self, AuthUser},
    db,
    error::AppError,
    routes::AppState,
};

#[derive(Deserialize, Serialize)]
pub struct CreateUserBody {
    pub username: String,
    pub password: String,
    pub role: String,
}

#[derive(Deserialize, Serialize)]
pub struct UpdateRoleBody {
    pub role: String,
}

pub async fn list_admin_users(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    auth::require_superadmin(&auth.0)?;
    let users = db::list_admin_users(&state.pool)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(json!(users)))
}

pub async fn create_admin_user(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Json(body): Json<CreateUserBody>,
) -> Result<Json<Value>, AppError> {
    auth::require_superadmin(&auth.0)?;

    if !matches!(body.role.as_str(), "ADMIN" | "MODERATOR" | "SUPERADMIN") {
        return Err(AppError::BadRequest("invalid role".into()));
    }

    let hash = bcrypt::hash(&body.password, 12).map_err(|e| AppError::Internal(e.to_string()))?;

    let id = db::create_admin_user(&state.pool, &body.username, &hash, &body.role)
        .await
        .map_err(|e| {
            if e.to_string().contains("Duplicate") {
                AppError::Conflict(format!("username '{}' already exists", body.username))
            } else {
                AppError::Internal(e.to_string())
            }
        })?;

    Ok(Json(
        json!({"id": id, "username": body.username, "role": body.role}),
    ))
}

pub async fn update_admin_user_role(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
    Json(body): Json<UpdateRoleBody>,
) -> Result<Json<Value>, AppError> {
    auth::require_superadmin(&auth.0)?;

    if !matches!(body.role.as_str(), "ADMIN" | "MODERATOR" | "SUPERADMIN") {
        return Err(AppError::BadRequest("invalid role".into()));
    }

    db::update_admin_user_role(&state.pool, id, &body.role)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(json!({"updated": true})))
}

pub async fn delete_admin_user(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    auth::require_superadmin(&auth.0)?;
    db::delete_admin_user(&state.pool, id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(json!({"deleted": true})))
}
