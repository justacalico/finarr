//! Indexer CRUD + connectivity tests.

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::auth::AdminUser;
use crate::error::ApiError;
use crate::indexers::{self, torznab::TorznabClient};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct IndexerBody {
    pub name: String,
    pub url: String,
    pub api_key: Option<String>,
    pub enabled: Option<bool>,
    pub categories: Option<Vec<u32>>,
    pub priority: Option<i64>,
}

pub async fn list(
    user: crate::auth::AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let mut items = indexers::list(&state.db)
        .await
        .map_err(ApiError::internal)?;
    // Indexer keys are secrets; non-admins get the list but not the keys.
    if user.role != "admin" {
        for i in &mut items {
            i.api_key = "********".into();
        }
    }
    Ok(Json(json!({ "indexers": items })))
}

pub async fn create(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Json(body): Json<IndexerBody>,
) -> Result<Json<Value>, ApiError> {
    if body.name.trim().is_empty() || body.url.trim().is_empty() {
        return Err(ApiError::bad_request("name and url required"));
    }
    let idx = indexers::create(
        &state.db,
        &indexers::IndexerConfig {
            name: body.name.trim(),
            url: body.url.trim(),
            api_key: body.api_key.as_deref().unwrap_or(""),
            enabled: body.enabled.unwrap_or(true),
            categories: &body.categories.unwrap_or_default(),
            priority: body.priority.unwrap_or(25),
        },
    )
    .await
    .map_err(ApiError::internal)?;
    Ok(Json(json!({ "indexer": idx })))
}

pub async fn update(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<IndexerBody>,
) -> Result<Json<Value>, ApiError> {
    // A masked key means "unchanged": keep the stored one instead of
    // persisting asterisks.
    let api_key = match body.api_key.as_deref() {
        Some("********") => indexers::get(&state.db, id)
            .await
            .map_err(ApiError::internal)?
            .map(|i| i.api_key)
            .unwrap_or_default(),
        Some(k) => k.to_string(),
        None => String::new(),
    };
    indexers::update(
        &state.db,
        id,
        &indexers::IndexerConfig {
            name: body.name.trim(),
            url: body.url.trim(),
            api_key: &api_key,
            enabled: body.enabled.unwrap_or(true),
            categories: &body.categories.unwrap_or_default(),
            priority: body.priority.unwrap_or(25),
        },
    )
    .await
    .map_err(ApiError::internal)?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn remove(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    indexers::delete(&state.db, id)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "ok": true })))
}

/// Probe an indexer that is already saved.
pub async fn test(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let idx = indexers::get(&state.db, id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("indexer not found"))?;
    test_client(&state, &idx.url, &idx.api_key).await
}

/// Probe unsaved credentials (used by the add-indexer dialog).
pub async fn test_new(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Json(body): Json<IndexerBody>,
) -> Result<Json<Value>, ApiError> {
    test_client(&state, &body.url, body.api_key.as_deref().unwrap_or("")).await
}

async fn test_client(
    state: &Arc<AppState>,
    url: &str,
    api_key: &str,
) -> Result<Json<Value>, ApiError> {
    let client = TorznabClient::new(url, api_key, state.http.clone());
    match client.caps().await {
        Ok(caps) => Ok(Json(json!({
            "ok": true,
            "caps": caps,
        }))),
        Err(e) => Ok(Json(json!({
            "ok": false,
            "error": e.to_string(),
        }))),
    }
}
