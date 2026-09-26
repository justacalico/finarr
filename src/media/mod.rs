//! Media library: movies, series (+seasons/episodes) and artists (+albums).

pub mod import;
pub mod naming;
pub mod parse;

use anyhow::{bail, Context, Result};
use serde::Serialize;
use sqlx::{FromRow, Row};
use std::path::Path;

use crate::db::Db;
use crate::metadata::{self, AlbumResult, MovieResult, SeriesResult};

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Movie {
    pub id: i64,
    pub tmdb_id: Option<i64>,
    pub imdb_id: Option<String>,
    pub title: String,
    pub sort_title: String,
    pub original_title: Option<String>,
    pub year: Option<i64>,
    pub overview: String,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub runtime_min: Option<i64>,
    pub genres: String,
    pub monitored: i64,
    pub path: String,
    pub status: String,
    pub added_at: String,
    pub release_date: Option<String>,
    pub digital_date: Option<String>,
    pub min_availability: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Series {
    pub id: i64,
    pub tvmaze_id: Option<i64>,
    pub tvdb_id: Option<i64>,
    pub tmdb_id: Option<i64>,
    pub imdb_id: Option<String>,
    pub title: String,
    pub sort_title: String,
    pub overview: String,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub year: Option<i64>,
    pub network: Option<String>,
    pub air_time: Option<String>,
    pub series_status: String,
    pub monitored: i64,
    pub path: String,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Season {
    pub id: i64,
    pub series_id: i64,
    pub season_number: i64,
    pub monitored: i64,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Episode {
    pub id: i64,
    pub series_id: i64,
    pub season_number: i64,
    pub episode_number: i64,
    pub title: String,
    pub overview: String,
    pub air_date: Option<String>,
    pub runtime_min: Option<i64>,
    pub monitored: i64,
    pub file_id: Option<i64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Artist {
    pub id: i64,
    pub mbid: Option<String>,
    pub name: String,
    pub sort_name: String,
    pub overview: String,
    pub image_url: Option<String>,
    pub monitored: i64,
    pub path: String,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Album {
    pub id: i64,
    pub artist_id: i64,
    pub mbid: Option<String>,
    pub title: String,
    pub release_date: Option<String>,
    pub album_type: String,
    pub monitored: i64,
    pub file_id: Option<i64>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct MediaFile {
    pub id: i64,
    pub path: String,
    pub size_bytes: i64,
    pub quality: String,
    pub movie_id: Option<i64>,
    pub episode_id: Option<i64>,
    pub album_id: Option<i64>,
    pub added_at: String,
}

fn sort_title(title: &str) -> String {
    for art in ["The ", "A ", "An "] {
        if let Some(rest) = title.strip_prefix(art) {
            return format!("{}, {}", rest, &art[..art.len() - 1]);
        }
    }
    title.to_string()
}

// ------------------------------ movies ------------------------------

pub async fn list_movies(db: &Db) -> Result<Vec<Movie>> {
    Ok(
        sqlx::query_as::<_, Movie>("SELECT * FROM movies ORDER BY sort_title")
            .fetch_all(db.pool())
            .await?,
    )
}

pub async fn get_movie(db: &Db, id: i64) -> Result<Movie> {
    Ok(
        sqlx::query_as::<_, Movie>("SELECT * FROM movies WHERE id = ?")
            .bind(id)
            .fetch_one(db.pool())
            .await?,
    )
}

/// Insert a movie from a fetched metadata result.
pub async fn add_movie(db: &Db, m: &MovieResult, root: &str, monitored: bool) -> Result<Movie> {
    let folder = naming::movie_folder(std::path::Path::new(root), &m.title, m.year);
    let path = folder.to_string_lossy().into_owned();
    let res = sqlx::query(
        "INSERT INTO movies (tmdb_id, imdb_id, title, sort_title, original_title, year,
            overview, poster_url, backdrop_url, runtime_min, genres, monitored, path,
            release_date, digital_date)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(m.tmdb_id)
    .bind(&m.imdb_id)
    .bind(&m.title)
    .bind(sort_title(&m.title))
    .bind(&m.original_title)
    .bind(m.year)
    .bind(&m.overview)
    .bind(&m.poster_url)
    .bind(&m.backdrop_url)
    .bind(m.runtime_min)
    .bind(serde_json::to_string(&m.genres)?)
    .bind(monitored as i64)
    .bind(&path)
    .bind(&m.release_date)
    .bind(&m.digital_date)
    .fetch_one(db.pool())
    .await;
    match res {
        Ok(row) => get_movie(db, row.get::<i64, _>("id")).await,
        Err(e) if e.to_string().contains("UNIQUE") => {
            bail!("movie already in library")
        }
        Err(e) => Err(e.into()),
    }
}

pub async fn update_movie_monitored(db: &Db, id: i64, monitored: bool) -> Result<()> {
    sqlx::query("UPDATE movies SET monitored = ? WHERE id = ?")
        .bind(monitored as i64)
        .bind(id)
        .execute(db.pool())
        .await?;
    Ok(())
}

/// Remove a directory only if it is strictly inside `root`: guards against
/// empty/malformed library paths resolving to the library root itself.
fn safe_remove_dir(path: &str, root: &Path) {
    let p = Path::new(path);
    if !path.is_empty() && p != root && p.starts_with(root) {
        let _ = std::fs::remove_dir_all(p);
    }
}

pub async fn delete_movie(db: &Db, id: i64, delete_files: bool, root: &Path) -> Result<()> {
    let movie = get_movie(db, id).await?;
    let files = movie_files(db, id).await?;
    sqlx::query("DELETE FROM movies WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await?;
    if delete_files {
        for f in &files {
            let _ = std::fs::remove_file(&f.path);
        }
        safe_remove_dir(&movie.path, root);
    }
    Ok(())
}

pub async fn movie_files(db: &Db, movie_id: i64) -> Result<Vec<MediaFile>> {
    Ok(
        sqlx::query_as::<_, MediaFile>(
            "SELECT * FROM media_files WHERE movie_id = ? ORDER BY path",
        )
        .bind(movie_id)
        .fetch_all(db.pool())
        .await?,
    )
}

// ------------------------------ series ------------------------------

pub async fn list_series(db: &Db) -> Result<Vec<Series>> {
    Ok(
        sqlx::query_as::<_, Series>("SELECT * FROM series ORDER BY sort_title")
            .fetch_all(db.pool())
            .await?,
    )
}

pub async fn get_series(db: &Db, id: i64) -> Result<Series> {
    Ok(
        sqlx::query_as::<_, Series>("SELECT * FROM series WHERE id = ?")
            .bind(id)
            .fetch_one(db.pool())
            .await?,
    )
}

pub async fn series_episodes(db: &Db, series_id: i64) -> Result<Vec<Episode>> {
    Ok(sqlx::query_as::<_, Episode>(
        "SELECT * FROM episodes WHERE series_id = ? ORDER BY season_number, episode_number",
    )
    .bind(series_id)
    .fetch_all(db.pool())
    .await?)
}

pub async fn series_seasons(db: &Db, series_id: i64) -> Result<Vec<Season>> {
    Ok(sqlx::query_as::<_, Season>(
        "SELECT * FROM seasons WHERE series_id = ? ORDER BY season_number",
    )
    .bind(series_id)
    .fetch_all(db.pool())
    .await?)
}

/// Insert a series with all seasons/episodes from the provider result.
/// `season_filter`: empty = monitor everything; otherwise only those
/// season numbers get monitored=1 (specials/0 only when requested).
pub async fn add_series(
    db: &Db,
    s: &SeriesResult,
    root: &str,
    season_filter: &[u32],
    monitored: bool,
) -> Result<Series> {
    let folder = naming::series_folder(std::path::Path::new(root), &s.title);
    let path = folder.to_string_lossy().into_owned();

    let exists: Option<i64> = sqlx::query("SELECT id FROM series WHERE tvmaze_id = ?")
        .bind(s.tvmaze_id)
        .fetch_optional(db.pool())
        .await?
        .map(|r| r.get::<i64, _>("id"));
    if exists.is_some() {
        bail!("series already in library");
    }

    let mut tx = db.pool().begin().await?;
    let row = sqlx::query(
        "INSERT INTO series (tvmaze_id, tvdb_id, tmdb_id, imdb_id, title, sort_title,
            overview, poster_url, backdrop_url, year, network, air_time,
            series_status, monitored, path)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(s.tvmaze_id)
    .bind(s.tvdb_id)
    .bind(None::<i64>)
    .bind(&s.imdb_id)
    .bind(&s.title)
    .bind(sort_title(&s.title))
    .bind(&s.overview)
    .bind(&s.poster_url)
    .bind(&s.backdrop_url)
    .bind(s.year)
    .bind(&s.network)
    .bind(&s.air_time)
    .bind(&s.status)
    .bind(monitored as i64)
    .bind(&path)
    .fetch_one(&mut *tx)
    .await?;
    let series_id: i64 = row.get("id");

    for season in &s.seasons {
        let monitored_season =
            monitored && (season_filter.is_empty() || season_filter.contains(&season.number));
        sqlx::query("INSERT INTO seasons (series_id, season_number, monitored) VALUES (?, ?, ?)")
            .bind(series_id)
            .bind(season.number as i64)
            .bind(monitored_season as i64)
            .execute(&mut *tx)
            .await?;
        for ep in &season.episodes {
            if ep.number == 0 {
                continue; // specials entries without a number
            }
            let status = match &ep.air_date {
                Some(d) if d.as_str() <= today().as_str() => "missing",
                _ => "unaired",
            };
            sqlx::query(
                "INSERT INTO episodes (series_id, season_number, episode_number, title,
                    overview, air_date, runtime_min, monitored, status)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(series_id)
            .bind(ep.season as i64)
            .bind(ep.number as i64)
            .bind(&ep.title)
            .bind(&ep.overview)
            .bind(&ep.air_date)
            .bind(ep.runtime_min)
            .bind(monitored_season as i64)
            .bind(status)
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    get_series(db, series_id).await
}

/// Re-pull provider metadata for a series and upsert new episodes.
pub async fn refresh_series(db: &Db, http: &reqwest::Client, series_id: i64) -> Result<usize> {
    let s = get_series(db, series_id).await?;
    let tvmaze_id = s
        .tvmaze_id
        .context("series has no tvmaze id, cannot refresh")?;
    let fresh = metadata::tvmaze::show(http, tvmaze_id).await?;

    sqlx::query(
        "UPDATE series SET title=?, overview=?, poster_url=?, backdrop_url=?,
            series_status=?, network=?, air_time=? WHERE id=?",
    )
    .bind(&fresh.title)
    .bind(&fresh.overview)
    .bind(&fresh.poster_url)
    .bind(&fresh.backdrop_url)
    .bind(&fresh.status)
    .bind(&fresh.network)
    .bind(&fresh.air_time)
    .bind(series_id)
    .execute(db.pool())
    .await?;

    let mut added = 0usize;
    for season in &fresh.seasons {
        let season_row: Option<i64> =
            sqlx::query("SELECT monitored FROM seasons WHERE series_id=? AND season_number=?")
                .bind(series_id)
                .bind(season.number as i64)
                .fetch_optional(db.pool())
                .await?
                .map(|r| r.get::<i64, _>("monitored"));
        let season_monitored = match season_row {
            Some(m) => m != 0,
            None => {
                // New season since last refresh: monitor it if the series is.
                let m = s.monitored != 0;
                sqlx::query(
                    "INSERT INTO seasons (series_id, season_number, monitored) VALUES (?, ?, ?)",
                )
                .bind(series_id)
                .bind(season.number as i64)
                .bind(m as i64)
                .execute(db.pool())
                .await?;
                m
            }
        };
        for ep in &season.episodes {
            if ep.number == 0 {
                continue;
            }
            let exists: Option<i64> = sqlx::query(
                "SELECT id FROM episodes WHERE series_id=? AND season_number=? AND episode_number=?",
            )
            .bind(series_id)
            .bind(ep.season as i64)
            .bind(ep.number as i64)
            .fetch_optional(db.pool())
            .await?
            .map(|r| r.get::<i64, _>("id"));
            if let Some(ep_id) = exists {
                sqlx::query(
                    "UPDATE episodes SET title=?, overview=?, air_date=?, runtime_min=? WHERE id=?",
                )
                .bind(&ep.title)
                .bind(&ep.overview)
                .bind(&ep.air_date)
                .bind(ep.runtime_min)
                .bind(ep_id)
                .execute(db.pool())
                .await?;
            } else {
                added += 1;
                let status = match &ep.air_date {
                    Some(d) if d.as_str() <= today().as_str() => "missing",
                    _ => "unaired",
                };
                sqlx::query(
                    "INSERT INTO episodes (series_id, season_number, episode_number, title,
                        overview, air_date, runtime_min, monitored, status)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(series_id)
                .bind(ep.season as i64)
                .bind(ep.number as i64)
                .bind(&ep.title)
                .bind(&ep.overview)
                .bind(&ep.air_date)
                .bind(ep.runtime_min)
                .bind(season_monitored as i64)
                .bind(status)
                .execute(db.pool())
                .await?;
            }
        }
    }
    // Episodes whose air date passed since last refresh become wanted.
    sqlx::query(
        "UPDATE episodes SET status='missing'
         WHERE series_id=? AND status='unaired' AND air_date IS NOT NULL AND air_date <= ?",
    )
    .bind(series_id)
    .bind(today())
    .execute(db.pool())
    .await?;
    Ok(added)
}

pub async fn set_season_monitored(
    db: &Db,
    series_id: i64,
    season: i64,
    monitored: bool,
) -> Result<()> {
    sqlx::query("UPDATE seasons SET monitored=? WHERE series_id=? AND season_number=?")
        .bind(monitored as i64)
        .bind(series_id)
        .bind(season)
        .execute(db.pool())
        .await?;
    sqlx::query("UPDATE episodes SET monitored=? WHERE series_id=? AND season_number=?")
        .bind(monitored as i64)
        .bind(series_id)
        .bind(season)
        .execute(db.pool())
        .await?;
    Ok(())
}

pub async fn set_episode_monitored(db: &Db, episode_id: i64, monitored: bool) -> Result<()> {
    sqlx::query("UPDATE episodes SET monitored=? WHERE id=?")
        .bind(monitored as i64)
        .bind(episode_id)
        .execute(db.pool())
        .await?;
    Ok(())
}

pub async fn update_series_monitored(db: &Db, id: i64, monitored: bool) -> Result<()> {
    sqlx::query("UPDATE series SET monitored=? WHERE id=?")
        .bind(monitored as i64)
        .bind(id)
        .execute(db.pool())
        .await?;
    Ok(())
}

pub async fn delete_series(db: &Db, id: i64, delete_files: bool, root: &Path) -> Result<()> {
    let s = get_series(db, id).await?;
    let files: Vec<MediaFile> = sqlx::query_as::<_, MediaFile>(
        "SELECT mf.* FROM media_files mf
         JOIN episodes e ON e.id = mf.episode_id WHERE e.series_id = ?",
    )
    .bind(id)
    .fetch_all(db.pool())
    .await?;
    sqlx::query("DELETE FROM series WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await?;
    if delete_files {
        for f in &files {
            let _ = std::fs::remove_file(&f.path);
        }
        safe_remove_dir(&s.path, root);
    }
    Ok(())
}

// ------------------------------ music ------------------------------

pub async fn list_artists(db: &Db) -> Result<Vec<Artist>> {
    Ok(
        sqlx::query_as::<_, Artist>("SELECT * FROM artists ORDER BY sort_name, name")
            .fetch_all(db.pool())
            .await?,
    )
}

pub async fn get_artist(db: &Db, id: i64) -> Result<Artist> {
    Ok(
        sqlx::query_as::<_, Artist>("SELECT * FROM artists WHERE id = ?")
            .bind(id)
            .fetch_one(db.pool())
            .await?,
    )
}

pub async fn artist_albums(db: &Db, artist_id: i64) -> Result<Vec<Album>> {
    Ok(sqlx::query_as::<_, Album>(
        "SELECT * FROM albums WHERE artist_id = ? ORDER BY release_date NULLS LAST",
    )
    .bind(artist_id)
    .fetch_all(db.pool())
    .await?)
}

pub async fn add_artist(
    db: &Db,
    artist: &metadata::ArtistResult,
    albums: &[AlbumResult],
    root: &str,
    monitored: bool,
) -> Result<Artist> {
    let folder = std::path::Path::new(root).join(naming::sanitize(&artist.name));
    let path = folder.to_string_lossy().into_owned();
    let exists: Option<i64> = sqlx::query("SELECT id FROM artists WHERE mbid = ?")
        .bind(&artist.mbid)
        .fetch_optional(db.pool())
        .await?
        .map(|r| r.get::<i64, _>("id"));
    if exists.is_some() {
        bail!("artist already in library");
    }
    let mut tx = db.pool().begin().await?;
    let row = sqlx::query(
        "INSERT INTO artists (mbid, name, sort_name, overview, image_url, monitored, path)
         VALUES (?, ?, ?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(&artist.mbid)
    .bind(&artist.name)
    .bind(if artist.sort_name.is_empty() {
        sort_title(&artist.name)
    } else {
        artist.sort_name.clone()
    })
    .bind(&artist.overview)
    .bind(&artist.image_url)
    .bind(monitored as i64)
    .bind(&path)
    .fetch_one(&mut *tx)
    .await?;
    let artist_id: i64 = row.get("id");
    for a in albums {
        // Only monitor real albums by default; EPs/singles/compilations
        // still get rows so the user can opt in.
        let auto = monitored && a.album_type == "album";
        sqlx::query(
            "INSERT INTO albums (artist_id, mbid, title, release_date, album_type, monitored)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(artist_id)
        .bind(&a.mbid)
        .bind(&a.title)
        .bind(&a.release_date)
        .bind(&a.album_type)
        .bind(auto as i64)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    get_artist(db, artist_id).await
}

pub async fn set_album_monitored(db: &Db, album_id: i64, monitored: bool) -> Result<()> {
    sqlx::query("UPDATE albums SET monitored=? WHERE id=?")
        .bind(monitored as i64)
        .bind(album_id)
        .execute(db.pool())
        .await?;
    Ok(())
}

pub async fn delete_artist(db: &Db, id: i64, delete_files: bool, root: &Path) -> Result<()> {
    let a = get_artist(db, id).await?;
    sqlx::query("DELETE FROM artists WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await?;
    if delete_files {
        safe_remove_dir(&a.path, root);
    }
    Ok(())
}

// ------------------------------ shared ------------------------------

pub async fn link_file(
    db: &Db,
    path: &str,
    size: i64,
    quality: &str,
    movie_id: Option<i64>,
    episode_id: Option<i64>,
    album_id: Option<i64>,
) -> Result<i64> {
    let row = sqlx::query(
        "INSERT INTO media_files (path, size_bytes, quality, movie_id, episode_id, album_id)
         VALUES (?, ?, ?, ?, ?, ?)
         ON CONFLICT(path) DO UPDATE SET
            size_bytes=excluded.size_bytes, quality=excluded.quality,
            movie_id=excluded.movie_id, episode_id=excluded.episode_id,
            album_id=excluded.album_id
         RETURNING id",
    )
    .bind(path)
    .bind(size)
    .bind(quality)
    .bind(movie_id)
    .bind(episode_id)
    .bind(album_id)
    .fetch_one(db.pool())
    .await?;
    Ok(row.get("id"))
}

pub fn today() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}
