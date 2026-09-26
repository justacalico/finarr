//! Background services: the sweeper that imports finished downloads and
//! the monitor that hunts wanted media on indexers.

use std::sync::Arc;
use std::time::Duration;

use sqlx::Row;

use crate::indexers::{self, torznab::SearchQuery};
use crate::settings::{self, AutomationSettings, EngineSettings, PathsSettings};
use crate::{grab, media, AppState};

/// Kick off every background task. Call once after AppState exists.
pub fn spawn_all(state: Arc<AppState>) {
    tokio::spawn(sweeper_loop(state.clone()));
    tokio::spawn(monitor_loop(state.clone()));
    tokio::spawn(refresh_loop(state));
}

/// Poll the engine, sync download_items rows, import what finished, and
/// enforce the seed-ratio limit.
async fn sweeper_loop(state: Arc<AppState>) {
    let mut interval = tokio::time::interval(Duration::from_secs(5));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        if let Err(e) = sweep_once(&state).await {
            tracing::warn!("sweep failed: {e:#}");
        }
    }
}

async fn sweep_once(state: &Arc<AppState>) -> anyhow::Result<()> {
    let engine = state.engine.read().await;
    let torrents = engine.list();
    let engine_settings = settings::get::<EngineSettings>(&state.db, "engine").await?;
    let paths = settings::get::<PathsSettings>(&state.db, "paths").await?;
    let automation = settings::get::<AutomationSettings>(&state.db, "automation").await?;
    let notifications = settings::get::<settings::NotificationSettings>(
        &state.db,
        "notifications",
    )
    .await?;

    // Seed ratio enforcement for everything the engine manages.
    if engine_settings.seed_ratio > 0.0 {
        for t in &torrents {
            if t.finished && t.total_bytes > 0 {
                let ratio = t.uploaded_bytes as f64 / t.total_bytes as f64;
                if ratio >= engine_settings.seed_ratio && t.state != "paused" {
                    let _ = engine.pause(&t.hash).await;
                }
            }
        }
    }

    let rows = sqlx::query(
        "SELECT id, hash, media_type, movie_id, episode_ids, album_id, save_path,
                release_title, imported, state
           FROM download_items WHERE client = 'builtin'",
    )
    .fetch_all(state.db.pool())
    .await?;

    for row in rows {
        let item_id: i64 = row.get("id");
        let hash: Option<String> = row.get("hash");
        let imported: i64 = row.get("imported");
        let Some(hash) = hash else { continue };
        let Some(t) = torrents.iter().find(|t| t.hash == hash) else {
            // Engine no longer knows it (deleted manually): mark removed.
            let _ = sqlx::query(
                "UPDATE download_items SET state='removed' WHERE id=? AND state!='imported'",
            )
            .bind(item_id)
            .execute(state.db.pool())
            .await;
            continue;
        };
        let new_state = if t.finished { "completed".to_string() } else { t.state.clone() };
        sqlx::query(
            "UPDATE download_items SET state=?, progress=?, size_bytes=?,
                save_path=COALESCE(NULLIF(save_path,''), ?)
             WHERE id=?",
        )
        .bind(&new_state)
        .bind(t.progress)
        .bind(t.total_bytes as i64)
        .bind(&t.save_path)
        .bind(item_id)
        .execute(state.db.pool())
        .await?;

        if t.finished && imported == 0 && automation.auto_import {
            let media_type: Option<String> = row.get("media_type");
            if media_type.is_none() {
                continue;
            }
            let src = std::path::PathBuf::from(if t.save_path.is_empty() {
                row.get::<String, _>("save_path")
            } else {
                t.save_path.clone()
            });
            let episode_ids: Vec<i64> =
                serde_json::from_str(&row.get::<String, _>("episode_ids")).unwrap_or_default();
            let outcome = media::import::import(
                &state.db,
                &paths,
                &src,
                media_type.as_deref().unwrap_or(""),
                row.get("movie_id"),
                &episode_ids,
                row.get("album_id"),
                &row.get::<String, _>("release_title"),
            )
            .await;
            match outcome {
                Ok(o) => {
                    sqlx::query(
                        "UPDATE download_items SET imported=1, state='imported' WHERE id=?",
                    )
                    .bind(item_id)
                    .execute(state.db.pool())
                    .await?;
                    let title: String = row.get("release_title");
                    state
                        .db
                        .record_history(
                            "imported",
                            media_type.as_deref().unwrap_or(""),
                            row.get::<Option<i64>, _>("movie_id")
                                .or(row.get::<Option<i64>, _>("album_id")),
                            &title,
                            serde_json::json!({ "files": o.files, "count": o.imported }),
                        )
                        .await?;
                    fulfill_requests(&state.db, media_type.as_deref().unwrap_or(""), &row).await?;
                    crate::notify::send(
                        &state.http,
                        &notifications,
                        "import",
                        "Import complete",
                        &format!("{} — {} file(s)", title, o.imported),
                    )
                    .await;
                }
                Err(e) => {
                    tracing::warn!("import of item {item_id} failed: {e:#}");
                    let _ = state
                        .db
                        .record_history(
                            "import_failed",
                            media_type.as_deref().unwrap_or(""),
                            row.get::<Option<i64>, _>("movie_id"),
                            &row.get::<String, _>("release_title"),
                            serde_json::json!({ "error": e.to_string() }),
                        )
                        .await;
                    // Do not retry in a hot loop; flag it so the user sees it.
                    sqlx::query(
                        "UPDATE download_items SET state='import_failed' WHERE id=?",
                    )
                    .bind(item_id)
                    .execute(state.db.pool())
                    .await?;
                }
            }
        }
    }
    Ok(())
}

/// Mark matching open requests fulfilled after an import.
async fn fulfill_requests(db: &crate::db::Db, media_type: &str, row: &sqlx::sqlite::SqliteRow) -> anyhow::Result<()> {
    let external_id = match media_type {
        "movie" => row
            .try_get::<Option<i64>, _>("movie_id")
            .ok()
            .flatten()
            .map(|id| id.to_string()),
        "album" => row
            .try_get::<Option<i64>, _>("album_id")
            .ok()
            .flatten()
            .map(|id| id.to_string()),
        _ => None,
    };
    if let Some(_) = external_id {
        // Requests store the provider id, not our row id, so match on the
        // media row's external id where we can.
        if media_type == "movie" {
            if let Some(mid) = row.try_get::<Option<i64>, _>("movie_id").ok().flatten() {
                if let Ok(m) = media::get_movie(db, mid).await {
                    if let Some(tmdb) = m.tmdb_id {
                        sqlx::query(
                            "UPDATE requests SET status='fulfilled', resolved_at=strftime('%Y-%m-%dT%H:%M:%fZ','now')
                             WHERE media_type='movie' AND external_id=? AND status IN ('pending','approved')",
                        )
                        .bind(tmdb.to_string())
                        .execute(db.pool())
                        .await?;
                    }
                }
            }
        }
    }
    Ok(())
}

/// Wanted-media search loop. Runs every `search_interval_min` minutes.
async fn monitor_loop(state: Arc<AppState>) {
    // Give the UI a moment to come up and any startup restore to settle.
    tokio::time::sleep(Duration::from_secs(20)).await;
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    let mut last_run = std::time::Instant::now() - Duration::from_secs(3600);
    loop {
        interval.tick().await;
        let automation: AutomationSettings =
            match settings::get(&state.db, "automation").await {
                Ok(a) => a,
                Err(_) => continue,
            };
        if !automation.wanted_search_enabled {
            continue;
        }
        let every = Duration::from_secs(automation.search_interval_min.max(1) * 60);
        if last_run.elapsed() < every {
            continue;
        }
        last_run = std::time::Instant::now();
        if let Err(e) = wanted_search(&state).await {
            tracing::warn!("wanted search failed: {e:#}");
        }
    }
}

/// One full pass over wanted media: search indexers, score, grab the best.
pub async fn wanted_search(state: &Arc<AppState>) -> anyhow::Result<usize> {
    let db = &state.db;
    let today = media::today();
    let mut grabbed = 0usize;

    // --- movies ---
    let movies: Vec<(i64, String, Option<i64>, Option<String>, Option<String>, Option<String>, String)> = sqlx::query_as(
        "SELECT id, title, year, imdb_id, release_date, digital_date, min_availability
           FROM movies WHERE monitored = 1 AND status = 'missing'",
    )
    .fetch_all(db.pool())
    .await?;
    for (id, title, year, imdb_id, release, digital, min_avail) in movies {
        if !movie_is_available(&release, &digital, &min_avail, &today) {
            continue;
        }
        let q = SearchQuery {
            kind: "movie".into(),
            query: match year {
                Some(y) => format!("{title} {y}"),
                None => title.clone(),
            },
            imdb_id,
            ..Default::default()
        };
        if let Some(best) = indexers::search_all(db, state.http.clone(), &q)
            .await
            .into_iter()
            .map(|r| {
                let s = indexers::score_release(&r, None, &[]);
                (s, r)
            })
            .filter(|(s, _)| *s > 0)
            .max_by_key(|(s, _)| *s)
        {
            if grab::grab(state, &best.1, "movie", Some(id), &[], None)
                .await
                .is_ok()
            {
                grabbed += 1;
            }
        }
    }

    // --- episodes (grouped by season so packs win when many are missing) ---
    let eps: Vec<(i64, i64, i64, i64, String)> = sqlx::query_as(
        "SELECT e.id, e.series_id, e.season_number, e.episode_number, s.title
           FROM episodes e JOIN series s ON s.id = e.series_id
          WHERE e.monitored = 1 AND e.status = 'missing'
            AND e.air_date IS NOT NULL AND e.air_date <= ?
          ORDER BY e.series_id, e.season_number",
    )
    .bind(&today)
    .fetch_all(db.pool())
    .await?;
    let mut by_season: std::collections::BTreeMap<(i64, i64), Vec<(i64, i64, String)>> =
        Default::default();
    for (eid, sid, season, epnum, title) in eps {
        by_season
            .entry((sid, season))
            .or_default()
            .push((eid, epnum, title));
    }
    for ((series_id, season), want) in by_season {
        let title = want[0].2.clone();
        let want_eps: Vec<u32> = want.iter().map(|w| w.1 as u32).collect();
        // Try a season pack first when the whole season is wanted.
        let whole_season_wanted = whole_season_missing(db, series_id, season).await;
        if whole_season_wanted {
            let q = SearchQuery {
                kind: "tvsearch".into(),
                query: title.clone(),
                season: Some(season as u32),
                ..Default::default()
            };
            if let Some((_, rel)) = best_of(
                indexers::search_all(db, state.http.clone(), &q).await,
                Some(season as u32),
                &want_eps,
            )
            .await
            {
                let ids: Vec<i64> = want.iter().map(|w| w.0).collect();
                if grab::grab(state, &rel, "series", None, &ids, None)
                    .await
                    .is_ok()
                {
                    grabbed += 1;
                    continue;
                }
            }
        }
        // Per-episode search for whatever remains.
        for (eid, epnum, _) in &want {
            let q = SearchQuery {
                kind: "tvsearch".into(),
                query: title.clone(),
                season: Some(season as u32),
                episode: Some(*epnum as u32),
                ..Default::default()
            };
            if let Some((_, rel)) = best_of(
                indexers::search_all(db, state.http.clone(), &q).await,
                Some(season as u32),
                &[*epnum as u32],
            )
            .await
            {
                if grab::grab(state, &rel, "series", None, &[*eid], None)
                    .await
                    .is_ok()
                {
                    grabbed += 1;
                }
            }
        }
    }

    // --- albums ---
    let albums: Vec<(i64, String, String, Option<String>)> = sqlx::query_as(
        "SELECT a.id, a.title, ar.name, a.release_date
           FROM albums a JOIN artists ar ON ar.id = a.artist_id
          WHERE a.monitored = 1 AND a.status = 'missing'
            AND (a.release_date IS NULL OR a.release_date <= ?)",
    )
    .bind(&today)
    .fetch_all(db.pool())
    .await?;
    for (aid, album, artist, _date) in albums {
        let q = SearchQuery {
            kind: "music".into(),
            query: format!("{artist} {album}"),
            ..Default::default()
        };
        if let Some(best) = indexers::search_all(db, state.http.clone(), &q)
            .await
            .into_iter()
            .map(|r| (indexers::score_release(&r, None, &[]), r))
            .filter(|(s, _)| *s > 0)
            .max_by_key(|(s, _)| *s)
        {
            if grab::grab(state, &best.1, "album", None, &[], Some(aid))
                .await
                .is_ok()
            {
                grabbed += 1;
            }
        }
    }
    Ok(grabbed)
}

async fn best_of(
    results: Vec<indexers::torznab::ReleaseResult>,
    season: Option<u32>,
    episodes: &[u32],
) -> Option<(i64, indexers::torznab::ReleaseResult)> {
    results
        .into_iter()
        .map(|r| (indexers::score_release(&r, season, episodes), r))
        .filter(|(s, _)| *s > 0)
        .max_by_key(|(s, _)| *s)
}

async fn whole_season_missing(db: &crate::db::Db, series_id: i64, season: i64) -> bool {
    let row = sqlx::query(
        "SELECT COUNT(*) AS missing, (SELECT COUNT(*) FROM episodes
          WHERE series_id=? AND season_number=? AND monitored=1) AS total
         FROM episodes WHERE series_id=? AND season_number=? AND monitored=1
           AND status='missing' AND air_date IS NOT NULL AND air_date <= ?",
    )
    .bind(series_id)
    .bind(season)
    .bind(series_id)
    .bind(season)
    .bind(media::today())
    .fetch_one(db.pool())
    .await;
    match row {
        Ok(r) => {
            let missing: i64 = r.get("missing");
            let total: i64 = r.get("total");
            total > 2 && missing * 2 >= total
        }
        Err(_) => false,
    }
}

/// Is a missing movie eligible for search? Respects min_availability.
fn movie_is_available(
    release: &Option<String>,
    digital: &Option<String>,
    min_avail: &str,
    today: &str,
) -> bool {
    match min_avail {
        "announced" => true,
        "in_cinemas" => release
            .as_deref()
            .map(|d| d <= today)
            .unwrap_or(false),
        // "released": any known date that passed counts; unknown dates stay
        // wanted (indexers will only return what exists anyway).
        _ => match (release, digital) {
            (None, None) => true,
            _ => {
                let r_ok = release.as_deref().map(|d| d <= today).unwrap_or(false);
                let d_ok = digital.as_deref().map(|d| d <= today).unwrap_or(false);
                r_ok || d_ok
            }
        },
    }
}

/// Periodically refresh continuing series so new seasons/episodes appear.
async fn refresh_loop(state: Arc<AppState>) {
    tokio::time::sleep(Duration::from_secs(60)).await;
    let mut interval = tokio::time::interval(Duration::from_secs(6 * 3600));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        let ids: Vec<i64> = match sqlx::query_scalar::<_, i64>(
            "SELECT id FROM series WHERE series_status = 'continuing' AND monitored = 1",
        )
        .fetch_all(state.db.pool())
        .await
        {
            Ok(v) => v,
            Err(_) => continue,
        };
        for id in ids {
            match media::refresh_series(&state.db, &state.http, id).await {
                Ok(added) if added > 0 => {
                    tracing::info!("series {id}: {added} new episode(s)");
                }
                Ok(_) => {}
                Err(e) => tracing::warn!("series {id} refresh failed: {e:#}"),
            }
        }
    }
}
