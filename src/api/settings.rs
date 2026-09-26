//! Settings sections, all stored in the DB and edited from the Web UI.

use axum::extract::{Path, State};
use axum::Json;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::auth::AdminUser;
use crate::error::ApiError;
use crate::settings::{self, EngineSettings, GeneralSettings};
use crate::AppState;

const SECTIONS: &[&str] = &[
    "general",
    "paths",
    "engine",
    "metadata",
    "automation",
    "notifications",
];

pub async fn all(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let mut map = serde_json::Map::new();
    for key in SECTIONS {
        let raw = state
            .db
            .get_setting(key)
            .await
            .map_err(ApiError::internal)?
            .unwrap_or_else(|| "null".into());
        map.insert(
            key.to_string(),
            serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null),
        );
    }
    // Fill missing sections with defaults so the UI always gets a full shape.
    ensure(&mut map, "general", &GeneralSettings::default());
    ensure(&mut map, "paths", &settings::PathsSettings::default());
    ensure(&mut map, "engine", &EngineSettings::default());
    ensure(&mut map, "metadata", &settings::MetadataSettings::default());
    ensure(&mut map, "automation", &settings::AutomationSettings::default());
    ensure(&mut map, "notifications", &settings::NotificationSettings::default());
    Ok(Json(Value::Object(map)))
}

fn ensure<T: serde::Serialize>(map: &mut serde_json::Map<String, Value>, key: &str, v: &T) {
    if map.get(key).map(|v| v.is_null()).unwrap_or(true) {
        map.insert(key.into(), serde_json::to_value(v).unwrap_or(Value::Null));
    }
}

pub async fn get_one(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(section): Path<String>,
) -> Result<Json<Value>, ApiError> {
    if !SECTIONS.contains(&section.as_str()) {
        return Err(ApiError::not_found("unknown settings section"));
    }
    let raw = state
        .db
        .get_setting(&section)
        .await
        .map_err(ApiError::internal)?;
    match raw {
        Some(v) => Ok(Json(serde_json::from_str(&v).unwrap_or(Value::Null))),
        None => Ok(Json(Value::Null)),
    }
}

pub async fn put(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
    Path(section): Path<String>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, ApiError> {
    // Validate against the typed shape before storing so a malformed PUT
    // cannot wedge the server.
    match section.as_str() {
        "general" => validate::<GeneralSettings>(&body)?,
        "paths" => validate::<settings::PathsSettings>(&body)?,
        "engine" => validate::<EngineSettings>(&body)?,
        "metadata" => validate::<settings::MetadataSettings>(&body)?,
        "automation" => validate::<settings::AutomationSettings>(&body)?,
        "notifications" => validate::<settings::NotificationSettings>(&body)?,
        _ => return Err(ApiError::not_found("unknown settings section")),
    }
    state
        .db
        .set_setting(&section, &body.to_string())
        .await
        .map_err(ApiError::internal)?;

    // Live-apply what can be applied without restart.
    if section == "engine" {
        let e: EngineSettings = serde_json::from_value(body).unwrap_or_default();
        let engine = state.engine.read().await;
        engine.set_limits(e.download_kbps, e.upload_kbps);
    }
    Ok(Json(json!({ "ok": true })))
}

fn validate<T: serde::de::DeserializeOwned>(v: &Value) -> Result<(), ApiError> {
    serde_json::from_value::<T>(v.clone())
        .map(|_| ())
        .map_err(|e| ApiError::bad_request(format!("invalid settings: {e}")))
}

/// Restart the BitTorrent session so listen/dht changes take effect.
pub async fn restart_engine(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    state
        .restart_engine()
        .await
        .map_err(|e| ApiError::bad_request(format!("engine restart failed: {e}")))?;
    Ok(Json(json!({ "ok": true })))
}
