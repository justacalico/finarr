//! User management (admin) and API keys.

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;

use crate::auth::{self, password, AdminUser};
use crate::error::ApiError;
use crate::AppState;

pub async fn list(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, username, display_name, role, disabled, created_at, last_login_at
           FROM users ORDER BY username",
    )
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::internal)?;
    let users: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.get::<i64,_>("id"),
                "username": r.get::<String,_>("username"),
                "display_name": r.get::<String,_>("display_name"),
                "role": r.get::<String,_>("role"),
                "disabled": r.get::<i64,_>("disabled") != 0,
                "created_at": r.get::<String,_>("created_at"),
                "last_login_at": r.get::<Option<String>,_>("last_login_at"),
            })
        })
        .collect();
    Ok(Json(json!({ "users": users })))
}

#[derive(Debug, Deserialize)]
pub struct CreateBody {
    pub username: String,
    pub password: String,
    pub display_name: Option<String>,
    pub role: Option<String>,
}

pub async fn create(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateBody>,
) -> Result<Json<Value>, ApiError> {
    if body.username.trim().is_empty() {
        return Err(ApiError::bad_request("username required"));
    }
    if body.password.len() < 8 {
        return Err(ApiError::bad_request(
            "password must be at least 8 characters",
        ));
    }
    let role = match body.role.as_deref() {
        Some("admin") => "admin",
        _ => "user",
    };
    let hash = password::hash_password(&body.password).map_err(ApiError::internal)?;
    let res = sqlx::query(
        "INSERT INTO users (username, password_hash, display_name, role)
         VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(body.username.trim())
    .bind(hash)
    .bind(body.display_name.unwrap_or_else(|| body.username.clone()))
    .bind(role)
    .fetch_one(state.db.pool())
    .await;
    match res {
        Ok(row) => Ok(Json(json!({ "id": row.get::<i64,_>("id") }))),
        Err(e) if e.to_string().contains("UNIQUE") => Err(ApiError::conflict("username taken")),
        Err(e) => Err(ApiError::internal(e)),
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateBody {
    pub display_name: Option<String>,
    pub role: Option<String>,
    pub disabled: Option<bool>,
    pub password: Option<String>,
}

pub async fn update(
    admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateBody>,
) -> Result<Json<Value>, ApiError> {
    if let Some(name) = &body.display_name {
        sqlx::query("UPDATE users SET display_name=? WHERE id=?")
            .bind(name)
            .bind(id)
            .execute(state.db.pool())
            .await
            .map_err(ApiError::internal)?;
    }
    if let Some(role) = &body.role {
        if !matches!(role.as_str(), "admin" | "user") {
            return Err(ApiError::bad_request("role must be admin or user"));
        }
        if admin.0.id == id && role != "admin" {
            return Err(ApiError::bad_request("cannot demote yourself"));
        }
        sqlx::query("UPDATE users SET role=? WHERE id=?")
            .bind(role)
            .bind(id)
            .execute(state.db.pool())
            .await
            .map_err(ApiError::internal)?;
    }
    if let Some(disabled) = body.disabled {
        if admin.0.id == id && disabled {
            return Err(ApiError::bad_request("cannot disable yourself"));
        }
        sqlx::query("UPDATE users SET disabled=? WHERE id=?")
            .bind(disabled as i64)
            .bind(id)
            .execute(state.db.pool())
            .await
            .map_err(ApiError::internal)?;
    }
    if let Some(pw) = &body.password {
        if pw.len() < 8 {
            return Err(ApiError::bad_request(
                "password must be at least 8 characters",
            ));
        }
        let hash = password::hash_password(pw).map_err(ApiError::internal)?;
        sqlx::query("UPDATE users SET password_hash=? WHERE id=?")
            .bind(hash)
            .bind(id)
            .execute(state.db.pool())
            .await
            .map_err(ApiError::internal)?;
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn remove(
    admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    if admin.0.id == id {
        return Err(ApiError::bad_request("cannot delete yourself"));
    }
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(state.db.pool())
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct ApiKeyBody {
    pub name: Option<String>,
}

/// Create a long-lived API key for this user (external tools, scripts).
pub async fn create_apikey(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<ApiKeyBody>,
) -> Result<Json<Value>, ApiError> {
    let exists: Option<i64> = sqlx::query("SELECT id FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(ApiError::internal)?
        .map(|r| r.get("id"));
    if exists.is_none() {
        return Err(ApiError::not_found("user not found"));
    }
    let token = auth::create_session(
        &state.db,
        id,
        body.name.as_deref().unwrap_or("api key"),
        "api_key",
        None,
    )
    .await
    .map_err(ApiError::internal)?;
    Ok(Json(json!({ "api_key": token })))
}
