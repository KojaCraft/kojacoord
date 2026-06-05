use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{
    auth::{self, AuthUser},
    error::AppError,
    routes::AppState,
};

const BACKUP_PREFIX: &str = "backups/";

const DOWNLOAD_EXPIRY_SECS: u64 = 15 * 60;

#[derive(Deserialize)]
pub struct BackupQuery {
    pub server: Option<String>,
}

pub async fn list_backups(
    State(state): State<Arc<AppState>>,
    _auth: AuthUser,
    Query(q): Query<BackupQuery>,
) -> Result<Json<Value>, AppError> {
    let prefix = match &q.server {
        Some(name) if !name.is_empty() => format!("{BACKUP_PREFIX}{name}/"),
        _ => BACKUP_PREFIX.to_owned(),
    };
    let objects = state
        .s3
        .list_detailed(&prefix)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(json!({ "backups": to_backup_list(objects) })))
}

pub async fn list_server_backups(
    State(state): State<Arc<AppState>>,
    _auth: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Value>, AppError> {
    let row = sqlx::query_scalar::<_, String>("SELECT name FROM servers WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("server not found".into()))?;

    let prefix = format!("{BACKUP_PREFIX}{row}/");
    let objects = state
        .s3
        .list_detailed(&prefix)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(
        json!({ "server": row, "backups": to_backup_list(objects) }),
    ))
}

#[derive(Deserialize)]
pub struct DownloadQuery {
    pub key: String,
}

pub async fn download_backup(
    State(state): State<Arc<AppState>>,
    _auth: AuthUser,
    Query(q): Query<DownloadQuery>,
) -> Result<Json<Value>, AppError> {
    if !q.key.starts_with(BACKUP_PREFIX) {
        return Err(AppError::BadRequest("not a backup key".into()));
    }
    let url = state
        .s3
        .presign_get(&q.key, DOWNLOAD_EXPIRY_SECS)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    Ok(Json(
        json!({ "url": url, "expires_in": DOWNLOAD_EXPIRY_SECS }),
    ))
}

pub async fn delete_backup(
    State(state): State<Arc<AppState>>,
    auth: AuthUser,
    Path(key): Path<String>,
) -> Result<Json<Value>, AppError> {
    auth::require_admin(&auth.0)?;
    if !key.starts_with(BACKUP_PREFIX) {
        return Err(AppError::BadRequest("not a backup key".into()));
    }
    state
        .s3
        .delete(&key)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;
    tracing::info!(by = %auth.0.sub, %key, "backup deleted");
    Ok(Json(json!({ "deleted": key })))
}

fn to_backup_list(objects: Vec<crate::s3::S3Object>) -> Vec<Value> {
    objects
        .iter()
        .filter(|o| !o.key.ends_with('/'))
        .map(|o| {
            let rel = o.key.strip_prefix(BACKUP_PREFIX).unwrap_or(&o.key);
            let mut parts = rel.splitn(2, '/');
            let server = parts.next().unwrap_or("").to_owned();
            let file = parts.next().unwrap_or(rel).to_owned();
            json!({
                "key":           o.key,
                "server":        server,
                "file":          file,
                "size":          o.size,
                "last_modified": o.last_modified,
            })
        })
        .collect()
}
