//! Calendar: upcoming episodes + movie releases in a date window.

use axum::extract::{Query, State};
use axum::Json;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::HashMap;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::AppState;

pub async fn calendar(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let today = crate::media::today();
    let start = params.get("start").cloned().unwrap_or_else(|| today.clone());
    let end = params
        .get("end")
        .cloned()
        .unwrap_or_else(|| {
            (chrono::Utc::now() + chrono::Duration::days(30))
                .format("%Y-%m-%d")
                .to_string()
        });

    let episodes: Vec<Value> = sqlx::query(
        "SELECT e.id, e.season_number, e.episode_number, e.title, e.air_date,
                e.status, s.id AS sid, s.title AS stitle, s.poster_url
           FROM episodes e JOIN series s ON s.id = e.series_id
          WHERE e.air_date IS NOT NULL AND e.air_date >= ? AND e.air_date <= ?
          ORDER BY e.air_date",
    )
    .bind(&start)
    .bind(&end)
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::internal)?
    .iter()
    .map(|r| {
        json!({
            "kind": "episode",
            "date": r.get::<String,_>("air_date"),
            "series_id": r.get::<i64,_>("sid"),
            "series_title": r.get::<String,_>("stitle"),
            "season": r.get::<i64,_>("season_number"),
            "episode": r.get::<i64,_>("episode_number"),
            "title": r.get::<String,_>("title"),
            "status": r.get::<String,_>("status"),
            "poster_url": r.get::<Option<String>,_>("poster_url"),
        })
    })
    .collect();

    let movies: Vec<Value> = sqlx::query(
        "SELECT id, title, year, poster_url, release_date, digital_date, status
           FROM movies WHERE monitored = 1 AND (
             (release_date IS NOT NULL AND release_date >= ? AND release_date <= ?)
             OR (digital_date IS NOT NULL AND digital_date >= ? AND digital_date <= ?))",
    )
    .bind(&start)
    .bind(&end)
    .bind(&start)
    .bind(&end)
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::internal)?
    .iter()
    .flat_map(|r| {
        let mut out = Vec::new();
        if let Some(d) = r.get::<Option<String>,_>("release_date") {
            out.push(json!({
                "kind": "movie", "subtype": "cinema", "date": d,
                "id": r.get::<i64,_>("id"),
                "title": r.get::<String,_>("title"),
                "year": r.get::<Option<i64>,_>("year"),
                "status": r.get::<String,_>("status"),
                "poster_url": r.get::<Option<String>,_>("poster_url"),
            }));
        }
        if let Some(d) = r.get::<Option<String>,_>("digital_date") {
            out.push(json!({
                "kind": "movie", "subtype": "digital", "date": d,
                "id": r.get::<i64,_>("id"),
                "title": r.get::<String,_>("title"),
                "year": r.get::<Option<i64>,_>("year"),
                "status": r.get::<String,_>("status"),
                "poster_url": r.get::<Option<String>,_>("poster_url"),
            }));
        }
        out
    })
    .collect();

    let albums: Vec<Value> = sqlx::query(
        "SELECT a.id, a.title, a.release_date, a.status, ar.name AS artist
           FROM albums a JOIN artists ar ON ar.id = a.artist_id
          WHERE a.monitored = 1 AND a.release_date IS NOT NULL
            AND a.release_date >= ? AND a.release_date <= ?",
    )
    .bind(&start)
    .bind(&end)
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::internal)?
    .iter()
    .map(|r| {
        json!({
            "kind": "album", "date": r.get::<String,_>("release_date"),
            "id": r.get::<i64,_>("id"),
            "title": format!("{} — {}", r.get::<String,_>("artist"), r.get::<String,_>("title")),
            "status": r.get::<String,_>("status"),
        })
    })
    .collect();

    let mut items: Vec<Value> = episodes.into_iter().chain(movies).chain(albums).collect();
    items.sort_by(|a, b| {
        let da = a.get("date").and_then(|v| v.as_str()).unwrap_or("");
        let db = b.get("date").and_then(|v| v.as_str()).unwrap_or("");
        da.cmp(db)
    });
    Ok(Json(json!({ "items": items, "start": start, "end": end })))
}
