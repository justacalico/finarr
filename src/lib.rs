//! Finarr, all-in-one media automation in a single binary.
//!
//! A BitTorrent client (librqbit), Torznab indexer search (Jackett and
//! friends), movie/series/music libraries with monitoring and automatic
//! import, and a request system, all behind the Flutter web UI that is
//! embedded right into this binary.

pub mod api;
pub mod auth;
pub mod config;
pub mod db;
pub mod download_clients;
pub mod error;
pub mod grab;
pub mod indexers;
pub mod logs;
pub mod media;
pub mod metadata;
pub mod notify;
pub mod services;
pub mod settings;
pub mod torrents;
pub mod web;

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use tower_http::compression::CompressionLayer;
use tower_http::cors::{Any, CorsLayer};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::trace::TraceLayer;

use settings::{EngineSettings, GeneralSettings, PathsSettings};

pub struct AppState {
    pub config: config::Config,
    pub db: db::Db,
    pub http: reqwest::Client,
    /// Swapped wholesale when engine settings change enough to need a
    /// fresh listen socket (port, dht). Rate limits apply live instead.
    pub engine: tokio::sync::RwLock<torrents::Engine>,
    /// Bumped on every engine restart so background tasks can detect they
    /// were looking at a dead session.
    pub engine_gen: std::sync::atomic::AtomicU64,
    /// The address the HTTP server actually bound (for the info page).
    pub listen_addr: Mutex<Option<String>>,
}

impl AppState {
    pub async fn restart_engine(&self) -> Result<()> {
        let paths = settings::get::<PathsSettings>(&self.db, "paths")
            .await?
            .resolve(&self.config.data_dir);
        let engine_settings = settings::get::<EngineSettings>(&self.db, "engine").await?;
        // Stop the old session first so the new one can keep the same port.
        // A brief window exists where readers hold a dead engine: they get
        // empty results rather than errors, and engine_gen lets sweepers
        // notice the swap instead of acting on stale data.
        let mut guard = self.engine.write().await;
        guard.session_stop().await;
        let new = torrents::Engine::start(
            &engine_settings,
            PathBuf::from(&paths.downloads_dir),
            self.config.torrent_state_dir(),
        )
        .await?;
        *guard = new;
        self.engine_gen
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(())
    }
}

async fn start_engine(state_db: &db::Db, cfg: &config::Config) -> Result<torrents::Engine> {
    let paths = settings::get::<PathsSettings>(state_db, "paths")
        .await?
        .resolve(&cfg.data_dir);
    let engine_settings = settings::get::<EngineSettings>(state_db, "engine").await?;
    torrents::Engine::start(
        &engine_settings,
        PathBuf::from(&paths.downloads_dir),
        cfg.torrent_state_dir(),
    )
    .await
}

pub async fn run(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<()> {
    logs::init();
    let cfg = config::parse_args(args)?;
    if !cfg.dev_mode {
        std::fs::create_dir_all(&cfg.data_dir).context("create data dir")?;
        std::fs::create_dir_all(cfg.torrent_state_dir()).ok();
    }
    let database = db::Db::connect(&cfg.db_url()).await?;
    let http = metadata::http_client();
    let engine = start_engine(&database, &cfg)
        .await
        .context("start torrent engine")?;
    let state = Arc::new(AppState {
        config: cfg.clone(),
        db: database,
        http,
        engine: tokio::sync::RwLock::new(engine),
        engine_gen: std::sync::atomic::AtomicU64::new(0),
        listen_addr: Mutex::new(None),
    });

    if cfg.dev_mode {
        // Dev bypasses auth as user 0, but rows like requests reference a
        // real user_id. Give dev mode a real admin account.
        if state.db.user_count().await.unwrap_or(0) == 0 {
            let hash =
                crate::auth::password::hash_password("dev-password-1234").unwrap_or_default();
            let _ = sqlx::query(
                "INSERT INTO users (username, password_hash, display_name, role)
                 VALUES ('dev', ?, 'Dev', 'admin')",
            )
            .bind(hash)
            .execute(state.db.pool())
            .await;
        }
    }
    crate::api::system::mark_start();
    services::spawn_all(state.clone());

    let api = api::router().with_state(state.clone());
    let app = axum::Router::new()
        .nest("/api", api)
        .route("/", axum::routing::get(web::index_handler))
        .route("/{*path}", axum::routing::get(web::static_handler))
        .layer(CompressionLayer::new())
        .layer(RequestBodyLimitLayer::new(64 * 1024 * 1024))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .layer(TraceLayer::new_for_http());

    let general = settings::get::<GeneralSettings>(&state.db, "general")
        .await
        .unwrap_or_default();
    let (host, port) = if cfg.dev_mode {
        let host: IpAddr = if cfg.dev_local {
            Ipv4Addr::LOCALHOST.into()
        } else {
            Ipv4Addr::UNSPECIFIED.into()
        };
        (host, 0)
    } else {
        let host: IpAddr = cfg.host_override.unwrap_or_else(|| {
            general
                .host
                .parse()
                .unwrap_or(IpAddr::V6(Ipv6Addr::UNSPECIFIED))
        });
        (host, cfg.port_override.unwrap_or(general.port))
    };

    let addr = SocketAddr::from((host, port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("bind {addr}"))?;
    let bound = listener.local_addr()?;
    *state.listen_addr.lock().unwrap() = Some(bound.to_string());
    tracing::info!("finarr listening on http://{bound}");

    let shutdown_state = state.clone();
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = tokio::signal::ctrl_c().await;
            tracing::info!("shutting down");
            let engine = shutdown_state.engine.read().await;
            engine.session_stop().await;
        })
        .await?;
    Ok(())
}
