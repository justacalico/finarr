//! Media requests (Seerr-style): users ask, admins approve, pipeline grabs.

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;

use crate::auth::{AdminUser, AuthUser};
use crate::error::ApiError;
use crate::media;
use crate::metadata::{musicbrainz, tmdb, tvmaze};
use crate::settings::{self, MetadataSettings, PathsSettings};
use crate::AppState;

pub async fn list(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let status = params.get("status").cloned().unwrap_or_default();
    let mut sql = String::from(
        "SELECT r.*, u.username AS requested_by_name FROM requests r
           JOIN users u ON u.id = r.requested_by",
    );
    if !status.is_empty() {
        sql.push_str(" WHERE r.status = ?");
    }
    sql.push_str(" ORDER BY r.created_at DESC LIMIT 500");
    let mut q = sqlx::query(&sql);
    if !status.is_empty() {
        q = q.bind(status);
    }
    let rows = q
        .fetch_all(state.db.pool())
        .await
        .map_err(ApiError::internal)?;
    let items: Vec<Value> = rows
        .iter()
        .map(|r| {
            json!({
                "id": r.get::<i64,_>("id"),
                "media_type": r.get::<String,_>("media_type"),
                "external_id": r.get::<String,_>("external_id"),
                "title": r.get::<String,_>("title"),
                "year": r.get::<Option<i64>,_>("year"),
                "poster_url": r.get::<Option<String>,_>("poster_url"),
                "detail": serde_json::from_str::<Value>(&r.get::<String,_>("detail")).unwrap_or(json!({})),
                "requested_by": r.get::<i64,_>("requested_by"),
                "requested_by_name": r.get::<String,_>("requested_by_name"),
                "status": r.get::<String,_>("status"),
                "note": r.get::<String,_>("note"),
                "created_at": r.get::<String,_>("created_at"),
                "resolved_at": r.get::<Option<String>,_>("resolved_at"),
            })
        })
        .collect();
    Ok(Json(json!({ "requests": items })))
}

#[derive(Debug, Deserialize)]
pub struct CreateBody {
    pub media_type: String,
    pub external_id: String,
    /// For series: which seasons to request. Empty = all.
    pub seasons: Option<Vec<u32>>,
    pub note: Option<String>,
}

pub async fn create(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(body): Json<CreateBody>,
) -> Result<Json<Value>, ApiError> {
    let paths = settings::get::<PathsSettings>(&state.db, "paths")
        .await
        .unwrap_or_default()
        .resolve(&state.config.data_dir);
    let (title, year, poster, mut detail) = match body.media_type.as_str() {
        "movie" => {
            let meta = settings::get::<MetadataSettings>(&state.db, "metadata")
                .await
                .unwrap_or_default();
            if meta.tmdb_api_key.trim().is_empty() {
                return Err(ApiError::bad_request("TMDB API key not configured"));
            }
            let tmdb_id: i64 = body
                .external_id
                .parse()
                .map_err(|_| ApiError::bad_request("external_id must be a tmdb id"))?;
            let m = tmdb::movie(&state.http, &meta.tmdb_api_key, tmdb_id)
                .await
                .map_err(|e| ApiError::bad_request(e.to_string()))?;
            {
                let d = serde_json::to_value(&m).unwrap();
                (m.title, m.year, m.poster_url, d)
            }
        }
        "series" => {
            let tvmaze_id: i64 = body
                .external_id
                .parse()
                .map_err(|_| ApiError::bad_request("external_id must be a tvmaze id"))?;
            let s = tvmaze::show(&state.http, tvmaze_id)
                .await
                .map_err(|e| ApiError::bad_request(e.to_string()))?;
            {
                let d = serde_json::to_value(&s).unwrap();
                (s.title, s.year, s.poster_url, d)
            }
        }
        "artist" => {
            let a = musicbrainz::artist(&state.http, &body.external_id)
                .await
                .map_err(|_| ApiError::bad_request("artist not found"))?;
            {
                let d = serde_json::to_value(&a).unwrap();
                (a.name.clone(), None, a.image_url.clone(), d)
            }
        }
        _ => {
            return Err(ApiError::bad_request(
                "media_type must be movie|series|artist",
            ))
        }
    };

    // Already in library? Fulfilled instantly.
    let already = match body.media_type.as_str() {
        "movie" => sqlx::query("SELECT id FROM movies WHERE tmdb_id = ?")
            .bind(body.external_id.parse::<i64>().unwrap_or(0))
            .fetch_optional(state.db.pool())
            .await
            .map_err(ApiError::internal)?
            .is_some(),
        "series" => sqlx::query("SELECT id FROM series WHERE tvmaze_id = ?")
            .bind(body.external_id.parse::<i64>().unwrap_or(0))
            .fetch_optional(state.db.pool())
            .await
            .map_err(ApiError::internal)?
            .is_some(),
        "artist" => sqlx::query("SELECT id FROM artists WHERE mbid = ?")
            .bind(&body.external_id)
            .fetch_optional(state.db.pool())
            .await
            .map_err(ApiError::internal)?
            .is_some(),
        _ => false,
    };
    if already {
        return Err(ApiError::conflict("already in library"));
    }

    if let Some(seasons) = &body.seasons {
        detail["seasons"] = json!(seasons);
    }
    // Admins' requests are auto-approved and kick off immediately.
    let status = if user.is_admin() {
        "approved"
    } else {
        "pending"
    };
    let row = sqlx::query(
        "INSERT INTO requests (media_type, external_id, title, year, poster_url, detail,
            requested_by, status, note, resolved_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, CASE WHEN ?='approved' THEN strftime('%Y-%m-%dT%H:%M:%fZ','now') END)
         RETURNING id",
    )
    .bind(&body.media_type)
    .bind(&body.external_id)
    .bind(&title)
    .bind(year)
    .bind(&poster)
    .bind(detail.to_string())
    .bind(user.id)
    .bind(status)
    .bind(body.note.unwrap_or_default())
    .bind(status)
    .fetch_one(state.db.pool())
    .await
    .map_err(ApiError::internal)?;
    let id: i64 = row.get("id");
    if status == "approved" {
        fulfill(&state, id, &paths).await;
    }
    Ok(Json(json!({ "id": id, "status": status })))
}

/// Turn an approved request into a monitored library row.
async fn fulfill(state: &Arc<AppState>, request_id: i64, paths: &PathsSettings) {
    let row = match sqlx::query("SELECT media_type, external_id, detail FROM requests WHERE id = ?")
        .bind(request_id)
        .fetch_optional(state.db.pool())
        .await
    {
        Ok(Some(r)) => r,
        _ => return,
    };
    let media_type: String = row.get("media_type");
    let external_id: String = row.get("external_id");
    let detail: Value = serde_json::from_str(&row.get::<String, _>("detail")).unwrap_or(json!({}));
    let result = match media_type.as_str() {
        "movie" => {
            let m: crate::metadata::MovieResult = match serde_json::from_value(detail) {
                Ok(m) => m,
                Err(_) => return,
            };
            media::add_movie(&state.db, &m, &paths.movies_root, true)
                .await
                .map(|_| ())
        }
        "series" => {
            let seasons: Vec<u32> = detail
                .get("seasons")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            let s: crate::metadata::SeriesResult = match serde_json::from_value(detail) {
                Ok(s) => s,
                Err(_) => return,
            };
            media::add_series(&state.db, &s, &paths.series_root, &seasons, true)
                .await
                .map(|_| ())
        }
        "artist" => {
            let albums = musicbrainz::albums(&state.http, &external_id)
                .await
                .unwrap_or_default();
            let a: crate::metadata::ArtistResult = match serde_json::from_value(detail) {
                Ok(a) => a,
                Err(_) => return,
            };
            media::add_artist(&state.db, &a, &albums, &paths.music_root, true)
                .await
                .map(|_| ())
        }
        _ => Ok(()),
    };
    if let Err(e) = result {
        tracing::warn!("fulfilling request {request_id} failed: {e:#}");
    }
}

pub async fn approve(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let res = sqlx::query(
        "UPDATE requests SET status='approved', resolved_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ? AND status = 'pending'",
    )
    .bind(id)
    .execute(state.db.pool())
    .await
    .map_err(ApiError::internal)?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("pending request not found"));
    }
    let paths = settings::get::<PathsSettings>(&state.db, "paths")
        .await
        .unwrap_or_default()
        .resolve(&state.config.data_dir);
    fulfill(&state, id, &paths).await;
    Ok(Json(json!({ "ok": true })))
}

pub async fn decline(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query(
        "UPDATE requests SET status='declined', resolved_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
         WHERE id = ? AND status = 'pending'",
    )
    .bind(id)
    .execute(state.db.pool())
    .await
    .map_err(ApiError::internal)?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn remove(
    user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    // Admins can delete anything; users only their own pending requests.
    let res = if user.is_admin() {
        sqlx::query("DELETE FROM requests WHERE id = ?")
            .bind(id)
            .execute(state.db.pool())
            .await
    } else {
        sqlx::query("DELETE FROM requests WHERE id = ? AND requested_by = ? AND status = 'pending'")
            .bind(id)
            .bind(user.id)
            .execute(state.db.pool())
            .await
    }
    .map_err(ApiError::internal)?;
    if res.rows_affected() == 0 {
        return Err(ApiError::not_found("request not found"));
    }
    Ok(Json(json!({ "ok": true })))
}
