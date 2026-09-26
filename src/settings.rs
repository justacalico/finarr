//! Typed application settings, persisted as JSON blobs in the `settings`
//! table. Everything is configured from the Web UI; these are the defaults
//! used before the first save.

use serde::{Deserialize, Serialize};

use crate::db::Db;
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralSettings {
    pub instance_name: String,
    /// Bind address for the HTTP server.
    pub host: String,
    pub port: u16,
    /// Show a small "server:" field on the login screen of hosted builds.
    pub allow_server_override: bool,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            instance_name: "Finarr".into(),
            host: "0.0.0.0".into(),
            port: 8787,
            allow_server_override: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PathsSettings {
    /// Where the built-in engine stores incomplete/complete data.
    pub downloads_dir: String,
    /// Library roots; media is imported into these folders.
    pub movies_root: String,
    pub series_root: String,
    pub music_root: String,
    /// hardlink | copy | move
    pub import_mode: String,
}

impl Default for PathsSettings {
    fn default() -> Self {
        Self {
            downloads_dir: "data/downloads".into(),
            movies_root: "data/library/movies".into(),
            series_root: "data/library/series".into(),
            music_root: "data/library/music".into(),
            import_mode: "hardlink".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EngineSettings {
    /// BitTorrent listen port for incoming peer connections.
    pub listen_port: u16,
    pub dht_enabled: bool,
    pub lsd_enabled: bool,
    /// Per-torrent peer cap. None = engine default.
    pub peer_limit: Option<usize>,
    /// Global rate limits in KiB/s. 0 = unlimited.
    pub download_kbps: u64,
    pub upload_kbps: u64,
    /// Stop seeding at this ratio. 0 = keep seeding forever.
    pub seed_ratio: f64,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            listen_port: 4242,
            dht_enabled: true,
            lsd_enabled: true,
            peer_limit: None,
            download_kbps: 0,
            upload_kbps: 0,
            seed_ratio: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct MetadataSettings {
    /// TMDB v3 API key. Required for movie metadata; series/music work
    /// without it via TVmaze and MusicBrainz.
    pub tmdb_api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AutomationSettings {
    /// Minutes between automatic searches for wanted media.
    pub search_interval_min: u64,
    pub auto_import: bool,
    pub wanted_search_enabled: bool,
}

impl Default for AutomationSettings {
    fn default() -> Self {
        Self {
            search_interval_min: 30,
            auto_import: true,
            wanted_search_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct NotificationSettings {
    /// ntfy | discord | gotify | webhook
    pub kind: String,
    /// ntfy: full topic URL. discord/webhook: webhook URL. gotify: base URL.
    pub url: String,
    /// gotify application token.
    pub token: String,
    pub priority: i32,
    pub events: Vec<String>,
}

impl NotificationSettings {
    pub fn enabled(&self) -> bool {
        !self.kind.is_empty() && !self.url.is_empty()
    }
}

/// Fetch a settings section, falling back to defaults on missing/invalid.
pub async fn get<T: for<'de> Deserialize<'de> + Default>(db: &Db, key: &str) -> Result<T> {
    match db.get_setting(key).await? {
        Some(raw) => Ok(serde_json::from_str(&raw).unwrap_or_default()),
        None => Ok(T::default()),
    }
}

/// Persist a settings section.
pub async fn set<T: Serialize>(db: &Db, key: &str, value: &T) -> Result<()> {
    db.set_setting(key, &serde_json::to_string(value)?).await
}

pub const KEY_GENERAL: &str = "general";
pub const KEY_PATHS: &str = "paths";
pub const KEY_ENGINE: &str = "engine";
pub const KEY_METADATA: &str = "metadata";
pub const KEY_AUTOMATION: &str = "automation";
pub const KEY_NOTIFICATIONS: &str = "notifications";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_roundtrip() {
        let g = GeneralSettings::default();
        let raw = serde_json::to_string(&g).unwrap();
        let back: GeneralSettings = serde_json::from_str(&raw).unwrap();
        assert_eq!(back.port, 8787);
        assert_eq!(back.host, "0.0.0.0");
    }

    #[test]
    fn partial_json_uses_defaults() {
        let p: PathsSettings = serde_json::from_str(r#"{"movies_root":"/media/m"}"#).unwrap();
        assert_eq!(p.movies_root, "/media/m");
        assert_eq!(p.import_mode, "hardlink");
    }

    #[test]
    fn notifications_disabled_by_default() {
        assert!(!NotificationSettings::default().enabled());
    }
}
