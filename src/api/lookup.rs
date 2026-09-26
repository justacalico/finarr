//! Metadata search endpoints backing the "add media" pickers.

use axum::extract::{Query, State};
use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::metadata::{musicbrainz, tmdb, tvmaze};
use crate::settings::{self, MetadataSettings};
use crate::AppState;

fn q(params: &HashMap<String, String>) -> Result<&String, ApiError> {
    params
        .get("q")
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| ApiError::bad_request("query param q required"))
}

pub async fn movies(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let query = q(&params)?;
    let meta = settings::get::<MetadataSettings>(&state.db, "metadata")
        .await
        .unwrap_or_default();
    if meta.tmdb_api_key.trim().is_empty() {
        return Err(ApiError::bad_request(
            "TMDB API key not configured (Settings > Metadata)",
        ));
    }
    let results = tmdb::search_movie(&state.http, &meta.tmdb_api_key, query)
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(json!({ "results": results })))
}

pub async fn series(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let query = q(&params)?;
    let results = tvmaze::search(&state.http, query)
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(json!({ "results": results })))
}

pub async fn artists(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<Value>, ApiError> {
    let query = q(&params)?;
    let results = musicbrainz::search_artists(&state.http, query)
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(json!({ "results": results })))
}
