//! Torrent management against the built-in engine.

use axum::extract::{Multipart, Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::AppState;

pub async fn list(_user: AuthUser, State(state): State<Arc<AppState>>) -> Json<Value> {
    let engine = state.engine.read().await;
    let items = engine.list();
    let (mut dl, mut ul) = (0u64, 0u64);
    for t in &items {
        dl += t.download_speed;
        ul += t.upload_speed;
    }
    Json(json!({
        "torrents": items,
        "speeds": { "download": dl, "upload": ul },
    }))
}

pub async fn one(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let engine = state.engine.read().await;
    let info = engine
        .get_info(&id)
        .map_err(|e| ApiError::not_found(e.to_string()))?;
    Ok(Json(json!({ "torrent": info })))
}

pub async fn files(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let engine = state.engine.read().await;
    let files = engine
        .files(&id)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(json!({ "files": files })))
}

#[derive(Debug, Deserialize)]
pub struct OnlyFilesBody {
    pub files: Vec<usize>,
}

pub async fn set_only_files(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(body): Json<OnlyFilesBody>,
) -> Result<Json<Value>, ApiError> {
    let engine = state.engine.read().await;
    engine
        .set_only_files(&id, &body.files)
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct AddJson {
    /// Magnet link, infohash, or URL to a .torrent file.
    pub url: String,
    pub category: Option<String>,
    pub paused: Option<bool>,
}

pub async fn add(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(body): Json<AddJson>,
) -> Result<Json<Value>, ApiError> {
    if body.url.trim().is_empty() {
        return Err(ApiError::bad_request("url required"));
    }
    let engine = state.engine.read().await;
    let (id, hash) = engine
        .add(
            body.url.trim(),
            None,
            body.category.as_deref().unwrap_or(""),
            body.paused.unwrap_or(false),
        )
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(json!({ "id": id, "hash": hash })))
}

/// Multipart upload: field `torrent` holds the .torrent file.
pub async fn add_file(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Result<Json<Value>, ApiError> {
    let mut file: Option<bytes::Bytes> = None;
    let mut category = String::new();
    let mut paused = false;
    while let Ok(Some(field)) = multipart.next_field().await {
        match field.name().unwrap_or("") {
            "torrent" => {
                file = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|_| ApiError::bad_request("bad file"))?,
                )
            }
            "category" => category = field.text().await.unwrap_or_default(),
            "paused" => {
                paused = field
                    .text()
                    .await
                    .map(|t| t == "true" || t == "1")
                    .unwrap_or(false)
            }
            _ => {}
        }
    }
    let Some(bytes) = file else {
        return Err(ApiError::bad_request("missing torrent file part"));
    };
    let engine = state.engine.read().await;
    let (id, hash) = engine
        .add("", Some(bytes), &category, paused)
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(json!({ "id": id, "hash": hash })))
}

pub async fn pause(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    state
        .engine
        .read()
        .await
        .pause(&id)
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn resume(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    state
        .engine
        .read()
        .await
        .resume(&id)
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct DeleteQuery {
    pub delete_files: Option<bool>,
}

pub async fn remove(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Query(q): Query<DeleteQuery>,
) -> Result<Json<Value>, ApiError> {
    state
        .engine
        .read()
        .await
        .delete(&id, q.delete_files.unwrap_or(false))
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct LimitsBody {
    pub download_kbps: u64,
    pub upload_kbps: u64,
}

pub async fn set_limits(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(body): Json<LimitsBody>,
) -> Result<Json<Value>, ApiError> {
    state
        .engine
        .read()
        .await
        .set_limits(body.download_kbps, body.upload_kbps);
    Ok(Json(json!({ "ok": true })))
}
