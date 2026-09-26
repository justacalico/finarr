//! Interactive release search ("search" buttons) + manual grab.

use axum::extract::{Path, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::grab;
use crate::indexers::{
    self,
    torznab::{ReleaseResult, SearchQuery},
};
use crate::media;
use crate::AppState;

/// Score + rank results for the UI so the best pick is on top.
fn scored(results: Vec<ReleaseResult>, season: Option<u32>, episodes: &[u32]) -> Vec<Value> {
    let mut out: Vec<Value> = results
        .into_iter()
        .map(|r| {
            let score = indexers::score_release(&r, season, episodes);
            let parsed = crate::media::parse::parse_release(&r.title);
            json!({
                "title": r.title,
                "guid": r.guid,
                "download_url": r.download_url,
                "info_hash": r.info_hash,
                "size_bytes": r.size_bytes,
                "seeders": r.seeders,
                "peers": r.peers,
                "grabs": r.grabs,
                "published": r.published,
                "indexer": r.indexer,
                "score": score,
                "quality": {
                    "source": parsed.source,
                    "resolution": parsed.resolution,
                    "season": parsed.season,
                    "episodes": parsed.episodes,
                    "season_pack": parsed.season_pack,
                    "proper": parsed.proper,
                    "repack": parsed.repack,
                },
            })
        })
        .collect();
    out.sort_by_key(|v| -(v.get("score").and_then(|s| s.as_i64()).unwrap_or(0)));
    out
}

pub async fn search_movie_releases(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let m = media::get_movie(&state.db, id)
        .await
        .map_err(|_| ApiError::not_found("movie not found"))?;
    let q = SearchQuery {
        kind: "movie".into(),
        query: match m.year {
            Some(y) => format!("{} {}", m.title, y),
            None => m.title.clone(),
        },
        imdb_id: m.imdb_id.clone(),
        ..Default::default()
    };
    let results = indexers::search_all(&state.db, state.http.clone(), &q).await;
    Ok(Json(json!({ "releases": scored(results, None, &[]) })))
}

pub async fn search_series_releases(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let s = media::get_series(&state.db, id)
        .await
        .map_err(|_| ApiError::not_found("series not found"))?;
    let q = SearchQuery {
        kind: "tvsearch".into(),
        query: s.title.clone(),
        ..Default::default()
    };
    let results = indexers::search_all(&state.db, state.http.clone(), &q).await;
    Ok(Json(json!({ "releases": scored(results, None, &[]) })))
}

pub async fn search_episode_releases(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let row = sqlx::query(
        "SELECT e.season_number, e.episode_number, s.title
           FROM episodes e JOIN series s ON s.id = e.series_id WHERE e.id = ?",
    )
    .bind(id)
    .fetch_optional(state.db.pool())
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("episode not found"))?;
    let season: i64 = row.get("season_number");
    let ep: i64 = row.get("episode_number");
    let q = SearchQuery {
        kind: "tvsearch".into(),
        query: row.get::<String, _>("title"),
        season: Some(season as u32),
        episode: Some(ep as u32),
        ..Default::default()
    };
    let results = indexers::search_all(&state.db, state.http.clone(), &q).await;
    Ok(Json(json!({
        "releases": scored(results, Some(season as u32), &[ep as u32])
    })))
}

pub async fn search_album_releases(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let row = sqlx::query(
        "SELECT a.title, ar.name AS artist FROM albums a
           JOIN artists ar ON ar.id = a.artist_id WHERE a.id = ?",
    )
    .bind(id)
    .fetch_optional(state.db.pool())
    .await
    .map_err(ApiError::internal)?
    .ok_or_else(|| ApiError::not_found("album not found"))?;
    let q = SearchQuery {
        kind: "music".into(),
        query: format!(
            "{} {}",
            row.get::<String, _>("artist"),
            row.get::<String, _>("title")
        ),
        ..Default::default()
    };
    let results = indexers::search_all(&state.db, state.http.clone(), &q).await;
    Ok(Json(json!({ "releases": scored(results, None, &[]) })))
}

#[derive(Debug, Deserialize)]
pub struct GrabBody {
    /// Which media this download feeds.
    pub media_type: String,
    pub movie_id: Option<i64>,
    pub episode_ids: Option<Vec<i64>>,
    pub album_id: Option<i64>,
    // Release fields echoed back from the search response.
    pub title: String,
    pub guid: Option<String>,
    pub download_url: String,
    pub info_hash: Option<String>,
    pub size_bytes: Option<u64>,
    pub seeders: Option<u32>,
    pub peers: Option<u32>,
    pub indexer: Option<String>,
}

pub async fn grab(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(body): Json<GrabBody>,
) -> Result<Json<Value>, ApiError> {
    let release = ReleaseResult {
        title: body.title.clone(),
        guid: body.guid.unwrap_or_default(),
        download_url: body.download_url.clone(),
        info_hash: body.info_hash,
        size_bytes: body.size_bytes.unwrap_or(0),
        seeders: body.seeders.unwrap_or(0),
        peers: body.peers.unwrap_or(0),
        grabs: 0,
        published: String::new(),
        categories: Vec::new(),
        indexer: body.indexer.unwrap_or_default(),
        score: 0,
    };
    let item = grab::grab(
        &state,
        &release,
        &body.media_type,
        body.movie_id,
        &body.episode_ids.unwrap_or_default(),
        body.album_id,
    )
    .await
    .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(json!({ "ok": true, "item": item })))
}
