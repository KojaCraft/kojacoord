use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::{auth::AuthUser, db, error::AppError, routes::AppState};

#[derive(Deserialize)]
pub struct LogFilter {
    pub server_id: Option<i64>,

    pub limit: Option<u32>,
}

pub async fn list_errors(
    State(state): State<Arc<AppState>>,
    _auth: AuthUser,
    Query(q): Query<LogFilter>,
) -> Result<Json<Value>, AppError> {
    let errors = db::list_errors(&state.pool, q.server_id, q.limit.unwrap_or(100)).await?;
    Ok(Json(json!(errors)))
}
