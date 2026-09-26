//! First-run wizard: create the admin account. Only usable while the
//! users table is empty — afterwards it is permanently locked.

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;

use crate::auth::{self, password};
use crate::error::ApiError;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct SetupBody {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
    /// Optional seed paths so the wizard can prefill Settings.
    pub paths: Option<serde_json::Value>,
}

pub async fn setup(
    State(state): State<Arc<AppState>>,
    Json(body): Json<SetupBody>,
) -> Result<Json<Value>, ApiError> {
    let count = state.db.user_count().await.map_err(ApiError::internal)?;
    if count > 0 && !state.config.dev_mode {
        return Err(ApiError::forbidden("setup already completed"));
    }
    if body.username.trim().is_empty() {
        return Err(ApiError::bad_request("username required"));
    }
    if body.password.len() < 8 {
        return Err(ApiError::bad_request("password must be at least 8 characters"));
    }
    let hash = password::hash_password(&body.password).map_err(ApiError::internal)?;
    let row = sqlx::query(
        "INSERT INTO users (username, password_hash, display_name, role)
         VALUES (?, ?, ?, 'admin') RETURNING id",
    )
    .bind(body.username.trim())
    .bind(hash)
    .bind(body.display_name.clone().unwrap_or_else(|| body.username.clone()))
    .fetch_one(state.db.pool())
    .await
    .map_err(ApiError::internal)?;
    let user_id: i64 = row.get("id");

    if let Some(paths) = body.paths {
        crate::settings::set(&state.db, "paths", &paths)
            .await
            .map_err(ApiError::internal)?;
    }

    let token = auth::create_session(&state.db, user_id, "setup", "session", Some(90))
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({
        "token": token,
        "user": { "id": user_id, "username": body.username, "role": "admin" }
    })))
}
