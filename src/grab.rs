//! The grab pipeline: take a chosen release, hand it to the right
//! download client, and record the linkage so the sweeper can import it.

use anyhow::{bail, Context, Result};
use sqlx::Row;

use crate::db::Db;
use crate::download_clients::{QbitClient, QbitConfig};
use crate::indexers::torznab::ReleaseResult;
use crate::AppState;

/// Which download client should take this release.
pub enum ClientChoice {
    Builtin,
    Qbit { id: i64, cfg: QbitConfig },
}

/// Pick the highest-priority enabled external client; the built-in engine
/// is the implicit default at priority 0 unless an external is preferred.
pub async fn pick_client(db: &Db) -> Result<ClientChoice> {
    let row = sqlx::query(
        "SELECT id, settings FROM download_clients WHERE enabled = 1 AND impl='qbittorrent' ORDER BY priority LIMIT 1",
    )
    .fetch_optional(db.pool())
    .await?;
    if let Some(row) = row {
        let cfg: QbitConfig =
            serde_json::from_str(&row.get::<String, _>("settings")).unwrap_or_default();
        return Ok(ClientChoice::Qbit {
            id: row.get("id"),
            cfg,
        });
    }
    Ok(ClientChoice::Builtin)
}

/// Fetch a .torrent file when the release link isn't a magnet.
async fn fetch_torrent(http: &reqwest::Client, url: &str) -> Result<Option<bytes::Bytes>> {
    if url.starts_with("magnet:") {
        return Ok(None);
    }
    let resp = http
        .get(url)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .with_context(|| format!("fetch torrent {url}"))?;
    if !resp.status().is_success() {
        bail!("torrent download failed: {}", resp.status());
    }
    Ok(Some(resp.bytes().await?))
}

/// Category name used for save-path grouping.
fn category_for(media_type: &str) -> &'static str {
    match media_type {
        "movie" => "movies",
        "series" => "series",
        "album" => "music",
        _ => "other",
    }
}

/// Send a release to the right client and register it in download_items
/// so completion triggers import of the linked media.
pub async fn grab(
    state: &AppState,
    release: &ReleaseResult,
    media_type: &str,
    movie_id: Option<i64>,
    episode_ids: &[i64],
    album_id: Option<i64>,
) -> Result<String> {
    let db = &state.db;
    let category = category_for(media_type);
    let (client_name, item_id, save_path) = match pick_client(db).await? {
        ClientChoice::Builtin => {
            let engine = state.engine.read().await;
            let bytes = fetch_torrent(&state.http, &release.download_url).await?;
            let (id, hash) = engine
                .add(&release.download_url, bytes, category, false)
                .await
                .context("add to engine")?;
            // Resolve the path rqbit will write into (category subdir).
            let save = engine
                .get_info(&id.to_string())
                .map(|i| i.save_path)
                .unwrap_or_default();
            ("builtin".to_string(), hash, save)
        }
        ClientChoice::Qbit { id, cfg } => {
            let client = QbitClient::login(&cfg).await.context("qbit login")?;
            if let Some(bytes) = fetch_torrent(&state.http, &release.download_url).await? {
                client.add_file(&bytes, "release.torrent").await?;
            } else {
                client.add_url(&release.download_url).await?;
            }
            let hash = release.info_hash.clone().unwrap_or_default();
            (format!("qbittorrent:{id}"), hash, String::new())
        }
    };

    sqlx::query(
        "INSERT INTO download_items (hash, client_item_id, client, name, media_type,
            movie_id, episode_ids, album_id, release_title, indexer, save_path,
            category, size_bytes, state)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 'downloading')",
    )
    .bind(&item_id)
    .bind(&item_id)
    .bind(&client_name)
    .bind(&release.title)
    .bind(media_type)
    .bind(movie_id)
    .bind(serde_json::to_string(episode_ids)?)
    .bind(album_id)
    .bind(&release.title)
    .bind(&release.indexer)
    .bind(&save_path)
    .bind(category)
    .bind(release.size_bytes as i64)
    .execute(db.pool())
    .await
    .context("record download item")?;

    // Mark linked entities so the UI shows progress.
    if let Some(mid) = movie_id {
        let _ = sqlx::query("UPDATE movies SET status='downloading' WHERE id=?")
            .bind(mid)
            .execute(db.pool())
            .await;
    }
    for eid in episode_ids {
        let _ = sqlx::query("UPDATE episodes SET status='downloading' WHERE id=?")
            .bind(eid)
            .execute(db.pool())
            .await;
    }
    if let Some(aid) = album_id {
        let _ = sqlx::query("UPDATE albums SET status='downloading' WHERE id=?")
            .bind(aid)
            .execute(db.pool())
            .await;
    }

    let what = match media_type {
        "movie" => "movie",
        "series" => "episodes",
        _ => "album",
    };
    db.record_history(
        "grabbed",
        media_type,
        movie_id
            .or(album_id)
            .or_else(|| episode_ids.first().copied()),
        &release.title,
        serde_json::json!({
            "release": release.title,
            "indexer": release.indexer,
            "client": client_name,
            "target": what,
        }),
    )
    .await?;
    Ok(item_id)
}
