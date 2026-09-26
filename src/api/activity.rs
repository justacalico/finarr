//! Activity: the live download queue merged with engine state + history.

use axum::extract::{Query, State};
use axum::Json;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::AppState;

/// Unified queue: engine torrents joined with tracked download_items so
/// the UI sees "Series S01E02 — 45%" rather than a bare torrent name.
pub async fn queue(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let engine = state.engine.read().await;
    let torrents = engine.list();
    let by_hash: HashMap<String, &crate::torrents::TorrentInfo> =
        torrents.iter().map(|t| (t.hash.clone(), t)).collect();

    let rows = sqlx::query(
        "SELECT di.*, m.title AS movie_title, a.title AS album_title
           FROM download_items di
           LEFT JOIN movies m ON m.id = di.movie_id
           LEFT JOIN albums a ON a.id = di.album_id
          ORDER BY di.added_at DESC LIMIT 200",
    )
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::internal)?;

    let tracked: Vec<Value> = rows
        .iter()
        .map(|r| {
            let hash: Option<String> = r.get("hash");
            let t = hash.as_ref().and_then(|h| by_hash.get(h));
            json!({
                "id": r.get::<i64,_>("id"),
                "name": r.get::<String,_>("name"),
                "hash": hash,
                "media_type": r.get::<Option<String>,_>("media_type"),
                "target_title": r.get::<Option<String>,_>("movie_title")
                    .or(r.get::<Option<String>,_>("album_title"))
                    .unwrap_or_default(),
                "state": r.get::<String,_>("state"),
                "progress": r.get::<f64,_>("progress"),
                "size_bytes": r.get::<i64,_>("size_bytes"),
                "client": r.get::<String,_>("client"),
                "indexer": r.get::<String,_>("indexer"),
                "added_at": r.get::<String,_>("added_at"),
                "live": t.map(|t| json!({
                    "download_speed": t.download_speed,
                    "upload_speed": t.upload_speed,
                    "eta_seconds": t.eta_seconds,
                    "peers": t.peers,
                    "engine_state": t.state,
                })),
            })
        })
        .collect();

    // Untracked engine torrents (added manually, not by automation).
    let tracked_hashes: std::collections::HashSet<String> = rows
        .iter()
        .filter_map(|r| r.get::<Option<String>, _>("hash"))
        .collect();
    let untracked: Vec<Value> = torrents
        .iter()
        .filter(|t| !tracked_hashes.contains(&t.hash))
        .map(|t| {
            json!({
                "id": null,
                "name": t.name,
                "hash": t.hash,
                "media_type": "manual",
                "target_title": "",
                "state": if t.finished { "completed".to_string() } else { t.state.clone() },
                "progress": t.progress,
                "size_bytes": t.total_bytes,
                "client": "builtin",
                "indexer": "",
                "added_at": "",
                "live": {
                    "download_speed": t.download_speed,
                    "upload_speed": t.upload_speed,
                    "eta_seconds": t.eta_seconds,
                    "peers": t.peers,
                    "engine_state": t.state,
                },
            })
        })
        .collect();

    let (mut dl, mut ul) = (0u64, 0u64);
    for t in &torrents {
        dl += t.download_speed;
        ul += t.upload_speed;
    }
    Ok(Json(json!({
        "items": tracked.into_iter().chain(untracked).collect::<Vec<_>>(),
        "speeds": { "download": dl, "upload": ul },
    })))
}

pub async fn history(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let limit = params
        .get("limit")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(100)
        .clamp(1, 1000);
    let rows = sqlx::query(
        "SELECT * FROM history ORDER BY created_at DESC, id DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::internal)?;
    let items: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.get::<i64,_>("id"),
                "event_type": r.get::<String,_>("event_type"),
                "media_type": r.get::<String,_>("media_type"),
                "ref_id": r.get::<Option<i64>,_>("ref_id"),
                "title": r.get::<String,_>("title"),
                "data": serde_json::from_str::<Value>(&r.get::<String,_>("data")).unwrap_or(json!({})),
                "created_at": r.get::<String,_>("created_at"),
            })
        })
        .collect();
    Ok(Json(json!({ "history": items })))
}
