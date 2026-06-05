use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    auth::{self, AuthUser},
    db,
    error::AppError,
    routes::AppState,
};

pub async fn list_servers(
    State(state): State<Arc<AppState>>,
    _auth: AuthUser,
) -> Result<Json<Value>, AppError> {
    let servers = db::list_servers(&state.pool).await?;

    let live = state.proxy.server_registry.all();

    let enriched: Vec<Value> = servers
        .iter()
        .map(|s| {
            let backend = live.iter().find(|b| b.name == s.name);
            json!({
                "id":                  s.id,
                "name":                s.name,
                "template":            s.template,

                "status":              backend
                    .map(|b| if b.is_online() { "running" } else { "stopped" })
                    .unwrap_or(s.status.as_str()),
                "address":             s.address,
                "port":                s.port,
                "docker_container_id": s.docker_container_id,
                "online":              backend.map(|b| b.is_online()).unwrap_or(false),
                "players_online":      backend.map(|b| b.player_count()).unwrap_or(0),
            })
        })
        .collect();

    Ok(Json(json!(enriched)))
}

pub async fn start_server(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    auth::require_admin(&auth.0)?;

    // Get server info from database
    let server = db::get_server_by_id(&state.pool, id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("server {}", id)))?;

    // Check if server backend exists and start it
    if let Some(backend) = state.proxy.server_registry.get(&server.name) {
        if backend.is_online() {
            return Err(AppError::BadRequest("Server is already running".into()));
        }
        // TODO: Implement actual server start logic via proxy
        tracing::info!(server_id = id, server_name = %server.name, "start_server requested");
    } else {
        return Err(AppError::NotFound(format!(
            "Server backend '{}' not found",
            server.name
        )));
    }

    Ok(Json(
        json!({"status": "starting", "server_id": id, "server_name": server.name}),
    ))
}

pub async fn stop_server(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    auth::require_admin(&auth.0)?;

    // Get server info from database
    let server = db::get_server_by_id(&state.pool, id)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::NotFound(format!("server {}", id)))?;

    // Check if server backend exists and stop it
    if let Some(backend) = state.proxy.server_registry.get(&server.name) {
        if !backend.is_online() {
            return Err(AppError::BadRequest("Server is already stopped".into()));
        }
        // TODO: Implement actual server stop logic via proxy
        tracing::info!(server_id = id, server_name = %server.name, "stop_server requested");
    } else {
        return Err(AppError::NotFound(format!(
            "Server backend '{}' not found",
            server.name
        )));
    }

    Ok(Json(
        json!({"status": "stopping", "server_id": id, "server_name": server.name}),
    ))
}
