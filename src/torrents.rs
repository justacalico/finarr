//! The built-in BitTorrent engine, backed by librqbit.

use std::net::{Ipv6Addr, SocketAddr};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{bail, Context, Result};
use librqbit::api::TorrentIdOrHash;
use librqbit::{
    AddTorrent, AddTorrentOptions, AddTorrentResponse, ManagedTorrent, Session,
    SessionOptions, SessionPersistenceConfig,
};
use serde::Serialize;

use crate::settings::EngineSettings;

#[derive(Debug, Clone, Serialize)]
pub struct TorrentInfo {
    pub id: usize,
    pub hash: String,
    pub name: String,
    /// initializing | live | paused | error
    pub state: String,
    pub finished: bool,
    pub progress: f64,
    pub progress_bytes: u64,
    pub total_bytes: u64,
    pub uploaded_bytes: u64,
    /// Bytes/sec.
    pub download_speed: u64,
    pub upload_speed: u64,
    pub eta_seconds: Option<u64>,
    pub peers: Option<PeerCounts>,
    pub save_path: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PeerCounts {
    pub live: u32,
    pub connecting: u32,
    pub queued: u32,
    pub seen: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TorrentFile {
    pub index: usize,
    pub path: String,
    pub size: u64,
    pub downloaded: u64,
    pub included: bool,
}

#[derive(Clone)]
pub struct Engine {
    session: Arc<Session>,
    downloads_dir: PathBuf,
}

impl Engine {
    pub async fn start(
        settings: &EngineSettings,
        downloads_dir: PathBuf,
        state_dir: PathBuf,
    ) -> Result<Self> {
        std::fs::create_dir_all(&downloads_dir).context("create downloads dir")?;
        std::fs::create_dir_all(&state_dir).context("create torrent state dir")?;

        let opts = SessionOptions {
            persistence: Some(SessionPersistenceConfig::Json {
                folder: Some(state_dir),
            }),
            listen: Some(librqbit::ListenerOptions {
                listen_addr: SocketAddr::from((Ipv6Addr::UNSPECIFIED, settings.listen_port)),
                enable_upnp_port_forwarding: true,
                ..Default::default()
            }),
            dht: if settings.dht_enabled {
                Some(librqbit::DhtSessionConfig::default())
            } else {
                None
            },
            disable_local_service_discovery: !settings.lsd_enabled,
            peer_limit: settings.peer_limit,
            fastresume: true,
            client_name_and_version: Some(format!("finarr {}", env!("CARGO_PKG_VERSION"))),
            ..Default::default()
        };

        let session = Session::new_with_opts(downloads_dir.clone(), opts)
            .await
            .context("start torrent session")?;

        let engine = Self {
            session,
            downloads_dir,
        };
        engine.set_limits(settings.download_kbps, settings.upload_kbps);
        Ok(engine)
    }

    /// Graceful shutdown: stop the session and all torrent tasks.
    pub async fn session_stop(&self) {
        self.session.stop().await;
    }

    pub fn set_limits(&self, download_kbps: u64, upload_kbps: u64) {
        self.session.ratelimits.set_download_bps(
            NonZeroU32::new((download_kbps as u32).saturating_mul(1024)),
        );
        self.session.ratelimits.set_upload_bps(
            NonZeroU32::new((upload_kbps as u32).saturating_mul(1024)),
        );
    }

    /// Safe subfolder under the downloads dir for a category name.
    fn category_dir(&self, category: &str) -> PathBuf {
        let clean: String = category
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        if clean.is_empty() {
            self.downloads_dir.clone()
        } else {
            self.downloads_dir.join(clean)
        }
    }

    /// Add a magnet/HTTP(S) torrent URL or raw .torrent bytes.
    /// Returns (torrent id, info hash hex).
    pub async fn add(
        &self,
        source: &str,
        file_bytes: Option<bytes::Bytes>,
        category: &str,
        paused: bool,
    ) -> Result<(usize, String)> {
        let add = if let Some(bytes) = file_bytes {
            AddTorrent::TorrentFileBytes(bytes)
        } else {
            AddTorrent::from_url(source.to_string())
        };
        let dir = self.category_dir(category);
        std::fs::create_dir_all(&dir).ok();
        let opts = AddTorrentOptions {
            paused,
            overwrite: true,
            output_folder: Some(dir.to_string_lossy().into_owned()),
            ..Default::default()
        };
        let resp = self.session.add_torrent(add, Some(opts)).await?;
        match resp {
            AddTorrentResponse::Added(id, h) | AddTorrentResponse::AlreadyManaged(id, h) => {
                Ok((id, h.info_hash().as_string()))
            }
            AddTorrentResponse::ListOnly(_) => bail!("unexpected list_only response"),
        }
    }

    fn lookup(&self, id_or_hash: &str) -> Result<Arc<ManagedTorrent>> {
        let id: TorrentIdOrHash = if let Ok(n) = id_or_hash.parse::<usize>() {
            TorrentIdOrHash::from(n)
        } else {
            TorrentIdOrHash::from(
                librqbit_core::Id20::from_str(id_or_hash)
                    .map_err(|_| anyhow::anyhow!("bad torrent id or hash"))?,
            )
        };
        self.session
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("torrent not found"))
    }

    pub fn list(&self) -> Vec<TorrentInfo> {
        self.session.with_torrents(|it| {
            it.map(|(id, t)| Self::info_of(id, t)).collect()
        })
    }

    fn info_of(id: usize, t: &Arc<ManagedTorrent>) -> TorrentInfo {
        let stats = t.stats();
        let (dl, ul, peers) = match &stats.live {
            Some(live) => (
                live.download_speed.as_bytes(),
                live.upload_speed.as_bytes(),
                Some(PeerCounts {
                    live: live.snapshot.peer_stats.live,
                    connecting: live.snapshot.peer_stats.connecting,
                    queued: live.snapshot.peer_stats.queued,
                    seen: live.snapshot.peer_stats.seen,
                }),
            ),
            None => (0, 0, None),
        };
        let remaining = stats.total_bytes.saturating_sub(stats.progress_bytes);
        let eta = if dl > 0 && remaining > 0 && !stats.finished {
            Some(remaining / dl.max(1))
        } else {
            None
        };
        TorrentInfo {
            id,
            hash: t.info_hash().as_string(),
            name: t.name().unwrap_or_else(|| "unnamed".into()),
            state: stats.state.to_string(),
            finished: stats.finished,
            progress: if stats.total_bytes > 0 {
                stats.progress_bytes as f64 / stats.total_bytes as f64
            } else {
                0.0
            },
            progress_bytes: stats.progress_bytes,
            total_bytes: stats.total_bytes,
            uploaded_bytes: stats.uploaded_bytes,
            download_speed: dl,
            upload_speed: ul,
            eta_seconds: eta,
            peers,
            save_path: t.output_folder().to_string_lossy().into_owned(),
            error: stats.error,
        }
    }

    pub fn get_info(&self, id_or_hash: &str) -> Result<TorrentInfo> {
        let t = self.lookup(id_or_hash)?;
        Ok(Self::info_of(t.id(), &t))
    }

    pub fn files(&self, id_or_hash: &str) -> Result<Vec<TorrentFile>> {
        let t = self.lookup(id_or_hash)?;
        let stats = t.stats();
        let only = t.only_files();
        t.with_metadata(|m| {
            m.file_infos
                .iter()
                .enumerate()
                .map(|(i, f)| TorrentFile {
                    index: i,
                    path: f.relative_filename.to_string_lossy().into_owned(),
                    size: f.len,
                    downloaded: stats.file_progress.get(i).copied().unwrap_or(0),
                    included: only.as_ref().map(|o| o.contains(&i)).unwrap_or(true),
                })
                .collect()
        })
    }

    pub async fn pause(&self, id_or_hash: &str) -> Result<()> {
        let t = self.lookup(id_or_hash)?;
        self.session.pause(&t).await
    }

    pub async fn resume(&self, id_or_hash: &str) -> Result<()> {
        let t = self.lookup(id_or_hash)?;
        self.session.unpause(&t).await
    }

    pub async fn delete(&self, id_or_hash: &str, delete_files: bool) -> Result<()> {
        let t = self.lookup(id_or_hash)?;
        self.session
            .delete(TorrentIdOrHash::from(t.id()), delete_files)
            .await
    }

    pub async fn set_only_files(&self, id_or_hash: &str, files: &[usize]) -> Result<()> {
        let t = self.lookup(id_or_hash)?;
        let set: std::collections::HashSet<usize> = files.iter().copied().collect();
        self.session.update_only_files(&t, &set).await
    }
}
