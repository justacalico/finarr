//! Public status + system info endpoints.

use axum::extract::{Query, State};
use axum::Json;
use serde::Serialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::auth::AuthUser;
use crate::error::ApiError;
use crate::settings::{self, GeneralSettings};
use crate::AppState;

#[derive(Serialize)]
pub struct StatusOut {
    pub version: String,
    pub setup_required: bool,
    pub instance_name: String,
    pub dev_mode: bool,
}

/// Public: is the first-run wizard needed?
pub async fn status(State(state): State<Arc<AppState>>) -> Result<Json<StatusOut>, ApiError> {
    let users = state.db.user_count().await.map_err(ApiError::internal)?;
    let general = settings::get::<GeneralSettings>(&state.db, "general")
        .await
        .unwrap_or_default();
    Ok(Json(StatusOut {
        version: env!("CARGO_PKG_VERSION").into(),
        setup_required: users == 0 && !state.config.dev_mode,
        instance_name: general.instance_name,
        dev_mode: state.config.dev_mode,
    }))
}

#[derive(Serialize)]
pub struct InfoOut {
    pub version: String,
    pub sqlite_version: String,
    pub data_dir: String,
    pub listen_addr: Option<String>,
    pub uptime_seconds: i64,
}

static START: std::sync::OnceLock<chrono::DateTime<chrono::Utc>> = std::sync::OnceLock::new();
pub fn mark_start() {
    let _ = START.set(chrono::Utc::now());
}

pub async fn info(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Json<InfoOut> {
    let sqlite_version = sqlx::query_scalar::<_, String>("SELECT sqlite_version()")
        .fetch_one(state.db.pool())
        .await
        .unwrap_or_default();
    let uptime = START
        .get()
        .map(|s| (chrono::Utc::now() - *s).num_seconds())
        .unwrap_or(0);
    Json(InfoOut {
        version: env!("CARGO_PKG_VERSION").into(),
        sqlite_version,
        data_dir: state.config.data_dir.to_string_lossy().into_owned(),
        listen_addr: state.listen_addr.lock().unwrap().clone(),
        uptime_seconds: uptime,
    })
}

pub async fn logs(
    _user: AuthUser,
    Query(q): Query<std::collections::HashMap<String, String>>,
) -> Json<Value> {
    let limit = q
        .get("limit")
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(200)
        .min(2000);
    Json(json!({ "entries": crate::logs::recent(limit) }))
}

#[derive(Serialize)]
pub struct DiskInfo {
    pub path: String,
    pub total_bytes: u64,
    pub free_bytes: u64,
}

pub async fn disk(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Json<Vec<DiskInfo>> {
    let paths = settings::get::<settings::PathsSettings>(&state.db, "paths")
        .await
        .unwrap_or_default();
    let mut out = Vec::new();
    for p in [
        paths.downloads_dir,
        paths.movies_root,
        paths.series_root,
        paths.music_root,
        state.config.data_dir.to_string_lossy().into_owned(),
    ] {
        let path = std::path::PathBuf::from(&p);
        if let Some(info) = disk_free(&path) {
            out.push(info);
        }
    }
    Json(out)
}

#[cfg(unix)]
fn disk_free(path: &std::path::Path) -> Option<DiskInfo> {
    use std::os::unix::ffi::OsStrExt;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    let c_path = std::ffi::CString::new(path.as_os_str().as_bytes()).ok()?;
    if unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) } != 0 {
        // Walk up until an existing ancestor answers.
        return path.parent().and_then(disk_free);
    }
    Some(DiskInfo {
        path: path.to_string_lossy().into_owned(),
        total_bytes: stat.f_blocks as u64 * stat.f_frsize as u64,
        free_bytes: stat.f_bavail as u64 * stat.f_frsize as u64,
    })
}

#[cfg(not(unix))]
fn disk_free(path: &std::path::Path) -> Option<DiskInfo> {
    Some(DiskInfo {
        path: path.to_string_lossy().into_owned(),
        total_bytes: 0,
        free_bytes: 0,
    })
}
