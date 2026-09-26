//! Image proxy: metadata providers host posters without reliable CORS,
//! and Flutter web can't load cross-origin images directly. The backend
//! fetches them and streams bytes with a permissive cache policy.
//! Only whitelisted provider hosts are proxied (no open proxy).

use axum::extract::{Query, State};
use axum::http::{header, HeaderValue};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::AppState;

const ALLOWED_HOSTS: &[&str] = &[
    "image.tmdb.org",
    "static.tvmaze.com",
    "coverartarchive.org",
    "archive.org",
];

pub async fn image(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Response, Json<Value>> {
    let Some(url) = params.get("url").cloned() else {
        return Err(Json(json!({"error": "url required"})));
    };
    let parsed = match reqwest::Url::parse(&url) {
        Ok(u) => u,
        Err(_) => return Err(Json(json!({"error": "bad url"}))),
    };
    let host = parsed.host_str().unwrap_or("");
    if !ALLOWED_HOSTS
        .iter()
        .any(|h| host == *h || host.ends_with(&format!(".{h}")))
    {
        return Err(Json(json!({"error": "host not allowed"})));
    }
    let resp = match state.http.get(parsed).send().await {
        Ok(r) => r,
        Err(e) => {
            return Err(Json(json!({"error": format!("fetch failed: {e}")})))
        }
    };
    if !resp.status().is_success() {
        return Err(Json(json!({"error": format!("upstream {}", resp.status())})));
    }
    let content_type = resp
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();
    let bytes = resp.bytes().await.unwrap_or_default();
    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_str(&content_type).unwrap(),
            ),
            (
                header::CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=86400"),
            ),
        ],
        bytes,
    )
        .into_response())
}
