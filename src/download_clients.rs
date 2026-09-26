//! External download clients. The built-in engine handles most needs;
//! qBittorrent support lets users keep an existing client.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct QbitConfig {
    /// e.g. "http://192.168.1.10:8080"
    pub host: String,
    pub username: String,
    pub password: String,
    /// Category applied to grabs (created automatically by qBittorrent).
    pub category: String,
}

impl Default for QbitConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            username: "admin".into(),
            password: String::new(),
            category: "finarr".into(),
        }
    }
}

pub struct QbitClient {
    cfg: QbitConfig,
    http: reqwest::Client,
}

#[derive(Debug, Deserialize)]
struct QbitTorrent {
    hash: String,
    name: String,
    state: String,
    progress: f64,
    size: i64,
    downloaded: i64,
    uploaded: i64,
    dlspeed: i64,
    upspeed: i64,
    eta: i64,
    save_path: Option<String>,
    category: Option<String>,
    num_seeds: Option<i64>,
    num_leechs: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemoteTorrent {
    pub hash: String,
    pub name: String,
    pub state: String,
    pub progress: f64,
    pub size_bytes: i64,
    pub downloaded: i64,
    pub uploaded: i64,
    pub download_speed: i64,
    pub upload_speed: i64,
    pub eta_seconds: i64,
    pub save_path: String,
    pub category: String,
    pub seeders: i64,
    pub leechers: i64,
}

impl QbitClient {
    fn base(&self) -> String {
        self.cfg.host.trim_end_matches('/').to_string()
    }

    /// Authenticate and return a client holding the SID cookie jar.
    pub async fn login(cfg: &QbitConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .cookie_store(true)
            .user_agent("finarr")
            .build()
            .context("build http client")?;
        let client = Self {
            cfg: cfg.clone(),
            http,
        };
        let resp = client
            .http
            .post(format!("{}/api/v2/auth/login", client.base()))
            .form(&[
                ("username", client.cfg.username.as_str()),
                ("password", client.cfg.password.as_str()),
            ])
            .send()
            .await
            .context("qbit login request")?;
        let body = resp.text().await.unwrap_or_default();
        if body.trim() != "Ok." {
            bail!("qBittorrent login failed (check username/password)");
        }
        Ok(client)
    }

    pub async fn torrents(&self) -> Result<Vec<RemoteTorrent>> {
        let items: Vec<QbitTorrent> = self
            .http
            .get(format!("{}/api/v2/torrents/info", self.base()))
            .send()
            .await
            .context("qbit list request")?
            .error_for_status()
            .context("qbit list status")?
            .json()
            .await
            .context("qbit list parse")?;
        Ok(items
            .into_iter()
            .map(|t| RemoteTorrent {
                hash: t.hash,
                name: t.name,
                state: t.state,
                progress: t.progress,
                size_bytes: t.size,
                downloaded: t.downloaded,
                uploaded: t.uploaded,
                download_speed: t.dlspeed,
                upload_speed: t.upspeed,
                eta_seconds: t.eta,
                save_path: t.save_path.unwrap_or_default(),
                category: t.category.unwrap_or_default(),
                seeders: t.num_seeds.unwrap_or(0),
                leechers: t.num_leechs.unwrap_or(0),
            })
            .collect())
    }

    /// Add a magnet link or torrent URL.
    pub async fn add_url(&self, url: &str) -> Result<()> {
        let resp = self
            .http
            .post(format!("{}/api/v2/torrents/add", self.base()))
            .form(&[("urls", url), ("category", self.cfg.category.as_str())])
            .send()
            .await
            .context("qbit add request")?;
        if !resp.status().is_success() {
            bail!("qBittorrent add failed: {}", resp.status());
        }
        Ok(())
    }

    pub async fn add_file(&self, bytes: &[u8], filename: &str) -> Result<()> {
        let part = reqwest::multipart::Part::bytes(bytes.to_vec()).file_name(filename.to_string());
        let form = reqwest::multipart::Form::new()
            .part("torrents", part)
            .text("category", self.cfg.category.clone());
        let resp = self
            .http
            .post(format!("{}/api/v2/torrents/add", self.base()))
            .multipart(form)
            .send()
            .await
            .context("qbit add-file request")?;
        if !resp.status().is_success() {
            bail!("qBittorrent add file failed: {}", resp.status());
        }
        Ok(())
    }

    async fn control(&self, action: &str, hash: &str, extra: &[(&str, &str)]) -> Result<()> {
        let mut form: Vec<(&str, &str)> = vec![("hashes", hash)];
        form.extend_from_slice(extra);
        let resp = self
            .http
            .post(format!("{}/api/v2/torrents/{action}", self.base()))
            .form(&form)
            .send()
            .await
            .context("qbit control request")?;
        if !resp.status().is_success() {
            bail!("qBittorrent {action} failed: {}", resp.status());
        }
        Ok(())
    }

    pub async fn pause(&self, hash: &str) -> Result<()> {
        self.control("stop", hash, &[]).await
    }

    pub async fn resume(&self, hash: &str) -> Result<()> {
        self.control("start", hash, &[]).await
    }

    pub async fn delete(&self, hash: &str, delete_files: bool) -> Result<()> {
        self.control(
            "delete",
            hash,
            &[("deleteFiles", if delete_files { "true" } else { "false" })],
        )
        .await
    }
}
