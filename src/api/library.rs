//! Library CRUD: movies, series (+episodes) and artists (+albums).

use axum::extract::{Path, Query, State};
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;

use crate::auth::{AdminUser, AuthUser};
use crate::error::ApiError;
use crate::media;
use crate::metadata::{musicbrainz, tmdb, tvmaze};
use crate::settings::{self, MetadataSettings, PathsSettings};
use crate::AppState;

// ------------------------------ movies ------------------------------

#[derive(Debug, Deserialize)]
pub struct AddMovieBody {
    pub tmdb_id: i64,
    pub monitored: Option<bool>,
}

pub async fn list_movies(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let items = media::list_movies(&state.db)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "movies": items })))
}

pub async fn get_movie(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let movie = media::get_movie(&state.db, id)
        .await
        .map_err(|_| ApiError::not_found("movie not found"))?;
    let files = media::movie_files(&state.db, id).await.unwrap_or_default();
    Ok(Json(json!({ "movie": movie, "files": files })))
}

pub async fn add_movie(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(body): Json<AddMovieBody>,
) -> Result<Json<Value>, ApiError> {
    let meta = settings::get::<MetadataSettings>(&state.db, "metadata")
        .await
        .unwrap_or_default();
    if meta.tmdb_api_key.trim().is_empty() {
        return Err(ApiError::bad_request(
            "TMDB API key not configured (Settings > Metadata)",
        ));
    }
    let paths = settings::get::<PathsSettings>(&state.db, "paths")
        .await
        .unwrap_or_default()
        .resolve(&state.config.data_dir);
    let details = tmdb::movie(&state.http, &meta.tmdb_api_key, body.tmdb_id)
        .await
        .map_err(|e| ApiError::bad_request(format!("tmdb lookup failed: {e}")))?;
    let movie = media::add_movie(
        &state.db,
        &details,
        &paths.movies_root,
        body.monitored.unwrap_or(true),
    )
    .await
    .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(json!({ "movie": movie })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateMovieBody {
    pub monitored: Option<bool>,
    pub min_availability: Option<String>,
}

pub async fn update_movie(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateMovieBody>,
) -> Result<Json<Value>, ApiError> {
    if let Some(m) = body.monitored {
        media::update_movie_monitored(&state.db, id, m)
            .await
            .map_err(ApiError::internal)?;
    }
    if let Some(avail) = &body.min_availability {
        sqlx::query("UPDATE movies SET min_availability = ? WHERE id = ?")
            .bind(avail)
            .bind(id)
            .execute(state.db.pool())
            .await
            .map_err(ApiError::internal)?;
    }
    Ok(Json(json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct DeleteQuery {
    pub delete_files: Option<bool>,
}

pub async fn delete_movie(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Query(q): Query<DeleteQuery>,
) -> Result<Json<Value>, ApiError> {
    media::delete_movie(&state.db, id, q.delete_files.unwrap_or(false))
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "ok": true })))
}

// ------------------------------ series ------------------------------

#[derive(Debug, Deserialize)]
pub struct AddSeriesBody {
    pub tvmaze_id: i64,
    /// Empty = all seasons monitored.
    pub seasons: Option<Vec<u32>>,
    pub monitored: Option<bool>,
}

pub async fn list_series(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let items = media::list_series(&state.db)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "series": items })))
}

pub async fn get_series(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let s = media::get_series(&state.db, id)
        .await
        .map_err(|_| ApiError::not_found("series not found"))?;
    let seasons = media::series_seasons(&state.db, id)
        .await
        .unwrap_or_default();
    Ok(Json(json!({ "series": s, "seasons": seasons })))
}

pub async fn add_series(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(body): Json<AddSeriesBody>,
) -> Result<Json<Value>, ApiError> {
    let paths = settings::get::<PathsSettings>(&state.db, "paths")
        .await
        .unwrap_or_default()
        .resolve(&state.config.data_dir);
    let details = tvmaze::show(&state.http, body.tvmaze_id)
        .await
        .map_err(|e| ApiError::bad_request(format!("tvmaze lookup failed: {e}")))?;
    let series = media::add_series(
        &state.db,
        &details,
        &paths.series_root,
        &body.seasons.unwrap_or_default(),
        body.monitored.unwrap_or(true),
    )
    .await
    .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(json!({ "series": series })))
}

pub async fn series_episodes(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let eps = media::series_episodes(&state.db, id)
        .await
        .map_err(ApiError::internal)?;
    let seasons = media::series_seasons(&state.db, id)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "episodes": eps, "seasons": seasons })))
}

pub async fn refresh_series(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let added = media::refresh_series(&state.db, &state.http, id)
        .await
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(json!({ "ok": true, "new_episodes": added })))
}

#[derive(Debug, Deserialize)]
pub struct UpdateSeriesBody {
    pub monitored: Option<bool>,
}

pub async fn update_series(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<UpdateSeriesBody>,
) -> Result<Json<Value>, ApiError> {
    if let Some(m) = body.monitored {
        media::update_series_monitored(&state.db, id, m)
            .await
            .map_err(ApiError::internal)?;
    }
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_series(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Query(q): Query<DeleteQuery>,
) -> Result<Json<Value>, ApiError> {
    media::delete_series(&state.db, id, q.delete_files.unwrap_or(false))
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
pub struct MonitorBody {
    pub monitored: bool,
}

pub async fn set_season_monitored(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<MonitorBody>,
) -> Result<Json<Value>, ApiError> {
    let row = sqlx::query("SELECT series_id, season_number FROM seasons WHERE id = ?")
        .bind(id)
        .fetch_one(state.db.pool())
        .await
        .map_err(|_| ApiError::not_found("season not found"))?;
    media::set_season_monitored(
        &state.db,
        row.get::<i64, _>("series_id"),
        row.get::<i64, _>("season_number"),
        body.monitored,
    )
    .await
    .map_err(ApiError::internal)?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn update_episode(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<MonitorBody>,
) -> Result<Json<Value>, ApiError> {
    media::set_episode_monitored(&state.db, id, body.monitored)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "ok": true })))
}

// ------------------------------ music ------------------------------

#[derive(Debug, Deserialize)]
pub struct AddArtistBody {
    pub mbid: String,
    pub monitored: Option<bool>,
}

pub async fn list_artists(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let items = media::list_artists(&state.db)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "artists": items })))
}

pub async fn get_artist(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let a = media::get_artist(&state.db, id)
        .await
        .map_err(|_| ApiError::not_found("artist not found"))?;
    let albums = media::artist_albums(&state.db, id)
        .await
        .unwrap_or_default();
    Ok(Json(json!({ "artist": a, "albums": albums })))
}

pub async fn add_artist(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Json(body): Json<AddArtistBody>,
) -> Result<Json<Value>, ApiError> {
    let paths = settings::get::<PathsSettings>(&state.db, "paths")
        .await
        .unwrap_or_default()
        .resolve(&state.config.data_dir);
    let albums = musicbrainz::albums(&state.http, &body.mbid)
        .await
        .map_err(|e| ApiError::bad_request(format!("musicbrainz lookup failed: {e}")))?;
    // Reuse the search-shaped struct; mbid is all we need stored.
    let artist = crate::metadata::ArtistResult {
        mbid: body.mbid.clone(),
        name: String::new(),
        sort_name: String::new(),
        overview: String::new(),
        image_url: None,
    };
    // The real name comes from the search endpoint; look it up once here.
    let artist = if let Ok(mut res) = musicbrainz::search_artists(&state.http, &body.mbid).await {
        res.pop().filter(|a| a.mbid == body.mbid).unwrap_or(artist)
    } else {
        artist
    };
    let artist = media::add_artist(
        &state.db,
        &artist,
        &albums,
        &paths.music_root,
        body.monitored.unwrap_or(true),
    )
    .await
    .map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(Json(json!({ "artist": artist })))
}

pub async fn artist_albums(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Value>, ApiError> {
    let albums = media::artist_albums(&state.db, id)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "albums": albums })))
}

pub async fn update_artist(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<MonitorBody>,
) -> Result<Json<Value>, ApiError> {
    sqlx::query("UPDATE artists SET monitored=? WHERE id=?")
        .bind(body.monitored as i64)
        .bind(id)
        .execute(state.db.pool())
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn delete_artist(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Query(q): Query<DeleteQuery>,
) -> Result<Json<Value>, ApiError> {
    media::delete_artist(&state.db, id, q.delete_files.unwrap_or(false))
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "ok": true })))
}

pub async fn update_album(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(body): Json<MonitorBody>,
) -> Result<Json<Value>, ApiError> {
    media::set_album_monitored(&state.db, id, body.monitored)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "ok": true })))
}

// ------------------------------ wanted ------------------------------

pub async fn wanted(
    _user: AuthUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let movies: Vec<Value> = sqlx::query(
        "SELECT id, title, year, poster_url, release_date, digital_date FROM movies
          WHERE monitored = 1 AND status = 'missing'",
    )
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::internal)?
    .iter()
    .map(|r| {
        json!({
            "kind": "movie", "id": r.get::<i64,_>("id"),
            "title": r.get::<String,_>("title"),
            "year": r.get::<Option<i64>,_>("year"),
            "poster_url": r.get::<Option<String>,_>("poster_url"),
            "date": r.get::<Option<String>,_>("digital_date")
                .or(r.get::<Option<String>,_>("release_date")),
        })
    })
    .collect();

    let episodes: Vec<Value> = sqlx::query(
        "SELECT e.id, e.series_id, e.season_number, e.episode_number, e.title,
                e.air_date, s.title AS stitle, s.poster_url
           FROM episodes e JOIN series s ON s.id = e.series_id
          WHERE e.monitored = 1 AND e.status = 'missing'
            AND (e.air_date IS NULL OR e.air_date <= strftime('%Y-%m-%d','now'))
          ORDER BY e.air_date DESC LIMIT 500",
    )
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::internal)?
    .iter()
    .map(|r| {
        json!({
            "kind": "episode", "id": r.get::<i64,_>("id"),
            "series_id": r.get::<i64,_>("series_id"),
            "season": r.get::<i64,_>("season_number"),
            "episode": r.get::<i64,_>("episode_number"),
            "title": format!("{} - S{:02}E{:02}", r.get::<String,_>("stitle"),
                r.get::<i64,_>("season_number"), r.get::<i64,_>("episode_number")),
            "episode_title": r.get::<String,_>("title"),
            "poster_url": r.get::<Option<String>,_>("poster_url"),
            "date": r.get::<Option<String>,_>("air_date"),
        })
    })
    .collect();

    let albums: Vec<Value> = sqlx::query(
        "SELECT a.id, a.title, a.release_date, ar.name AS artist, ar.image_url
           FROM albums a JOIN artists ar ON ar.id = a.artist_id
          WHERE a.monitored = 1 AND a.status = 'missing'
            AND (a.release_date IS NULL OR a.release_date <= strftime('%Y-%m-%d','now'))",
    )
    .fetch_all(state.db.pool())
    .await
    .map_err(ApiError::internal)?
    .iter()
    .map(|r| {
        json!({
            "kind": "album", "id": r.get::<i64,_>("id"),
            "title": format!("{}, {}", r.get::<String,_>("artist"), r.get::<String,_>("title")),
            "poster_url": r.get::<Option<String>,_>("image_url"),
            "date": r.get::<Option<String>,_>("release_date"),
        })
    })
    .collect();

    Ok(Json(json!({
        "movies": movies,
        "episodes": episodes,
        "albums": albums,
    })))
}

/// Rescan library folders: links on-disk files to episodes/movies that
/// predate Finarr. Conservative: only marks things imported when the file
/// name parses cleanly to the target.
pub async fn scan_library(
    _admin: AdminUser,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, ApiError> {
    let paths = settings::get::<PathsSettings>(&state.db, "paths")
        .await
        .unwrap_or_default()
        .resolve(&state.config.data_dir);
    let report = scan::run(&state.db, &paths)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(json!({ "report": report })))
}

pub mod scan {
    //! Existing-library scanner: walk the configured roots and link any
    //! parseable files to their media rows.

    use anyhow::Result;
    use serde::Serialize;
    use walkdir::WalkDir;

    use crate::db::Db;
    use crate::media::{self, naming, parse};
    use crate::settings::PathsSettings;

    #[derive(Debug, Serialize)]
    pub struct ScanReport {
        pub files_seen: usize,
        pub linked: usize,
        pub skipped: usize,
    }

    pub async fn run(db: &Db, paths: &PathsSettings) -> Result<ScanReport> {
        let mut report = ScanReport {
            files_seen: 0,
            linked: 0,
            skipped: 0,
        };

        // Episodes: match <root>/<SeriesDir>/Season NN/*.mkv file names to
        // series titles in the DB.
        let series = media::list_series(db).await?;
        let root = std::path::Path::new(&paths.series_root);
        if root.is_dir() {
            for dir in std::fs::read_dir(root)?.filter_map(|e| e.ok()) {
                let dirname = dir.file_name().to_string_lossy().to_lowercase();
                let Some(s) = series
                    .iter()
                    .find(|s| naming::sanitize(&s.title).to_lowercase() == dirname)
                else {
                    continue;
                };
                let episodes = media::series_episodes(db, s.id).await?;
                for entry in WalkDir::new(dir.path()).into_iter().filter_map(|e| e.ok()) {
                    let p = entry.path();
                    if !p.is_file() || !naming::is_video(p) || naming::is_sample_or_junk(p) {
                        continue;
                    }
                    report.files_seen += 1;
                    let fname = p
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    let parsed = parse::parse_release(&fname);
                    let hit = episodes.iter().find(|e| {
                        e.status != "imported"
                            && parse::matches_episode(
                                &parsed,
                                e.season_number as u32,
                                &[e.episode_number as u32],
                            )
                    });
                    match hit {
                        Some(ep) => {
                            let size = p.metadata().map(|m| m.len() as i64).unwrap_or(0);
                            let quality = parsed
                                .resolution
                                .map(|r| format!("{r}p"))
                                .unwrap_or_default();
                            let fid = media::link_file(
                                db,
                                &p.to_string_lossy(),
                                size,
                                &quality,
                                None,
                                Some(ep.id),
                                None,
                            )
                            .await?;
                            sqlx::query(
                                "UPDATE episodes SET status='imported', file_id=? WHERE id=?",
                            )
                            .bind(fid)
                            .bind(ep.id)
                            .execute(db.pool())
                            .await?;
                            report.linked += 1;
                        }
                        None => report.skipped += 1,
                    }
                }
            }
        }

        // Movies: <root>/<Title (YYYY)>/file
        let movies = media::list_movies(db).await?;
        let mroot = std::path::Path::new(&paths.movies_root);
        if mroot.is_dir() {
            for dir in std::fs::read_dir(mroot)?.filter_map(|e| e.ok()) {
                let dirname = dir.file_name().to_string_lossy().to_lowercase();
                let Some(m) = movies.iter().find(|m| {
                    let want = format!("{} ({})", naming::sanitize(&m.title), m.year.unwrap_or(0))
                        .to_lowercase();
                    dirname == want || dirname == naming::sanitize(&m.title).to_lowercase()
                }) else {
                    continue;
                };
                for entry in WalkDir::new(dir.path()).into_iter().filter_map(|e| e.ok()) {
                    let p = entry.path();
                    if !p.is_file() || !naming::is_video(p) || naming::is_sample_or_junk(p) {
                        continue;
                    }
                    report.files_seen += 1;
                    if m.status == "imported" {
                        report.skipped += 1;
                        continue;
                    }
                    let size = p.metadata().map(|m| m.len() as i64).unwrap_or(0);
                    let quality = parse::parse_release(
                        &p.file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                    )
                    .resolution
                    .map(|r| format!("{r}p"))
                    .unwrap_or_default();
                    media::link_file(
                        db,
                        &p.to_string_lossy(),
                        size,
                        &quality,
                        Some(m.id),
                        None,
                        None,
                    )
                    .await?;
                    sqlx::query("UPDATE movies SET status='imported', path=? WHERE id=?")
                        .bind(dir.path().to_string_lossy().into_owned())
                        .bind(m.id)
                        .execute(db.pool())
                        .await?;
                    report.linked += 1;
                }
            }
        }

        // Albums: <root>/<Artist>/<Album (...)>/
        let artists = media::list_artists(db).await?;
        let aroot = std::path::Path::new(&paths.music_root);
        if aroot.is_dir() {
            for artist in &artists {
                let artist_dir = naming::sanitize(&artist.name).to_lowercase();
                for entry in WalkDir::new(aroot).min_depth(2).max_depth(2) {
                    let Ok(e) = entry else { continue };
                    if !e.path().is_dir() {
                        continue;
                    }
                    let parent_matches = e
                        .path()
                        .parent()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().to_lowercase() == artist_dir)
                        .unwrap_or(false);
                    if !parent_matches {
                        continue;
                    }
                    let albums = media::artist_albums(db, artist.id).await?;
                    let dirname = e.file_name().to_string_lossy().to_lowercase();
                    let Some(album) = albums
                        .iter()
                        .find(|a| dirname.starts_with(&naming::sanitize(&a.title).to_lowercase()))
                    else {
                        continue;
                    };
                    let mut size = 0i64;
                    let mut seen = 0usize;
                    for f in WalkDir::new(e.path()).into_iter().filter_map(|e| e.ok()) {
                        if f.path().is_file() && naming::is_audio(f.path()) {
                            seen += 1;
                            size += f.metadata().map(|m| m.len() as i64).unwrap_or(0);
                            report.files_seen += 1;
                        }
                    }
                    if seen > 0 && album.status != "imported" {
                        let fid = media::link_file(
                            db,
                            &e.path().to_string_lossy(),
                            size,
                            "",
                            None,
                            None,
                            Some(album.id),
                        )
                        .await?;
                        sqlx::query("UPDATE albums SET status='imported', file_id=? WHERE id=?")
                            .bind(fid)
                            .bind(album.id)
                            .execute(db.pool())
                            .await?;
                        report.linked += seen;
                    }
                }
            }
        }
        Ok(report)
    }
}
