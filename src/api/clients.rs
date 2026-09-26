//! Download client management (external qBittorrent; the built-in engine
//! is always available and needs no row).

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;

use crate::auth::AdminUser;
use crate::download_clients::{QbitClient, QbitConfig};
use crate::error::ApiError;
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct ClientBody {
    pub name: String,
    pub host: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub category: Option<String>,
    pub priority: Option<i64>,
    pub enabled: Option<bool>,
}

fn cfg_of(b: &ClientBody) -> QbitConfig {
    QbitConfig {
        host: b.host.clone(),
        username: b.username.clone().unwrap_or_else(|| "admin".into()),
        password: b.password.clone().unwrap_or_default(),
        category: b.category.clone().unwrap_or_else(|| "finarr".into()),
    }
}

pub async fn list(
    _user: crate::auth::AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, name, impl, settings, priority, enabled FROM download_clients ORDER BY priority",
    )
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::internal)?;
    let clients: Vec<Value> = rows
        .iter()
        .map(|r| {
            let mut cfg: Value =
                serde_json::from_str(&r.get::<String, _>("settings")).unwrap_or(json!({}));
            // Never leak the stored password back to the UI.
            if let Some(obj) = cfg.as_object_mut() {
                if obj.get("password").and_then(|p| p.as_str()).map(|p| !p.is_empty()).unwrap_or(false) {
                    obj.insert("password".into(), json!("********"));
                }
            }
            json!({
                "id": r.get::<i64, _>("id"),
                "name": r.get::<String, _>("name"),
                "impl": r.get::<String, _>("impl"),
                "settings": cfg,
                "priority": r.get::<i64, _>("priority"),
                "enabled": r.get::<i64, _>("enabled") != 0,
            })
        })
        .collect();
    Ok(Json(json!({ "clients": clients, "builtin": true })))
}

pub async fn create(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Json(body): Json<ClientBody>,
) -> Result<Json<Value>, ApiError> {
    if body.name.trim().is_empty() || body.host.trim().is_empty() {
        return Err(ApiError::bad_request("name and host required"));
    }
    let cfg = cfg_of(&body);
    let row = sqlx::query(
        "INSERT INTO download_clients (name, impl, settings, priority, enabled)
         VALUES (?, 'qbittorrent', ?, ?, ?) RETURNING id",
    )
    .bind(body.name.trim())
    .bind(serde_json::to_string(&cfg).unwrap())
    .bind(body.priority.unwrap_or(1))
    .bind(body.enabled.unwrap_or(true) as i64)
    .fetch_one(state.db.pool())
    .await
    .map_err(ApiError::internal)?;
    Ok(Json(json!({ "id": row.get::<i64, _>("id") })))
}

pub async fn update(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<ClientBody>,
) -> Result<Json<Value>, ApiError> {
    let existing: Option<String> = sqlx::query("SELECT settings FROM download_clients WHERE id = ?")
        .bind(id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(ApiError::internal)?
        .map(|r| r.get::<String, _>("settings"));
    let Some(raw) = existing else {
        return Err(ApiError::not_found("client not found"));
    };
    let old: QbitConfig = serde_json::from_str(&raw).unwrap_or_default();
    let mut cfg = cfg_of(&body);
    // A masked password field means "keep the stored one".
    if cfg.password == "********" {
        cfg.password = old.password;
    }
    sqlx::query(
        "UPDATE download_clients SET name=?, settings=?, priority=?, enabled=? WHERE id=?",
    )
    .bind(body.name.trim())
    .bind(serde_json::to_string(&cfg).unwrap())
    .bind(body.priority.unwrap_or(1))
    .bind(body.enabled.unwrap_or(true) as i64)
    .bind(id)
    .execute(state.db.pool())
    .await
    .map_err(ApiError::internal)?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn remove(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("DELETE FROM download_clients WHERE id = ?")
        .bind(id)
        .execute(state.db.pool())
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn test(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let raw: Option<String> = sqlx::query("SELECT settings FROM download_clients WHERE id = ?")
        .bind(id)
        .fetch_optional(state.db.pool())
        .await
        .map_err(ApiError::internal)?
        .map(|r| r.get::<String, _>("settings"));
    let Some(raw) = raw else {
        return Err(ApiError::not_found("client not found"));
    };
    let cfg: QbitConfig = serde_json::from_str(&raw).unwrap_or_default();
    Ok(test_cfg(&cfg).await)
}

pub async fn test_new(
    _admin: AdminUser,
    Json(body): Json<ClientBody>,
) -> Result<Json<Value>, ApiError> {
    Ok(test_cfg(&cfg_of(&body)).await)
}

async fn test_cfg(cfg: &QbitConfig) -> Json<Value> {
    match QbitClient::login(cfg).await {
        Ok(client) => match client.torrents().await {
            Ok(ts) => Json(json!({ "ok": true, "torrents": ts.len() })),
            Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
        },
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}
