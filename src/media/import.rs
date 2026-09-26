//! Import completed downloads into the library with clean naming.
//! Prefers hardlinks so the torrent can keep seeding without doubling
//! disk usage; falls back to copy, or move when configured.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Serialize;
use walkdir::WalkDir;

use crate::db::Db;
use crate::media::{self, naming, parse};
use crate::settings::PathsSettings;

#[derive(Debug, Clone, Serialize)]
pub struct ImportOutcome {
    pub imported: usize,
    pub skipped: usize,
    pub files: Vec<String>,
}

/// Place `src` at `dest` using the configured mode, off the async executor:
/// multi-GB copies can't block a runtime worker.
async fn place_file(src: &Path, dest: &Path, mode: &str) -> Result<()> {
    let src = src.to_path_buf();
    let dest = dest.to_path_buf();
    let mode = mode.to_string();
    tokio::task::spawn_blocking(move || place_file_blocking(&src, &dest, &mode))
        .await
        .context("fs task")?
}

fn place_file_blocking(src: &Path, dest: &Path, mode: &str) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    match mode {
        "copy" => {
            std::fs::copy(src, dest).with_context(|| "copy failed")?;
        }
        "move" => {
            if std::fs::rename(src, dest).is_err() {
                std::fs::copy(src, dest)
                    .and_then(|_| std::fs::remove_file(src))
                    .with_context(|| "move failed")?;
            }
        }
        // hardlink, default: keeps the seed intact without extra space.
        _ => {
            if std::fs::hard_link(src, dest).is_err() {
                std::fs::copy(src, dest).with_context(|| "link/copy failed")?;
            }
        }
    }
    Ok(())
}

/// Media-looking files under a completed download dir.
async fn collect_files(dir: &Path, kind: &str) -> Vec<PathBuf> {
    let dir = dir.to_path_buf();
    let kind = kind.to_string();
    tokio::task::spawn_blocking(move || collect_files_blocking(&dir, &kind))
        .await
        .unwrap_or_default()
}

fn collect_files_blocking(dir: &Path, kind: &str) -> Vec<PathBuf> {
    // A single video/audio file is already the download itself.
    if dir.is_file() {
        let ok = match kind {
            "movie" | "series" => naming::is_video(dir),
            "album" => naming::is_audio(dir),
            _ => false,
        };
        return if ok {
            vec![dir.to_path_buf()]
        } else {
            Vec::new()
        };
    }
    let mut out: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let p = entry.path();
        if !p.is_file() || naming::is_sample_or_junk(p) {
            continue;
        }
        let matches = match kind {
            "movie" | "series" => naming::is_video(p),
            "album" => naming::is_audio(p),
            _ => false,
        };
        if matches {
            out.push(p.to_path_buf());
        }
    }
    // Largest first, the real feature is rarely the smallest file.
    out.sort_by_key(|p| std::cmp::Reverse(p.metadata().map(|m| m.len()).unwrap_or(0)));
    out
}

fn quality_of(name: &str) -> String {
    let p = parse::parse_release(name);
    match (p.resolution, p.source) {
        (Some(r), Some(s)) => format!("{s} {r}p"),
        (Some(r), None) => format!("{r}p"),
        (None, Some(s)) => s,
        _ => String::new(),
    }
}

/// What a finished download should become in the library.
pub enum ImportKind {
    Movie(i64),
    Series(Vec<i64>),
    Album(i64),
}

/// Import everything under `src_dir` for a tracked download item.
pub async fn import(
    db: &Db,
    paths: &PathsSettings,
    src_dir: &Path,
    kind: ImportKind,
    release_title: &str,
) -> Result<ImportOutcome> {
    if !src_dir.exists() {
        bail!("download path no longer exists: {}", src_dir.display());
    }
    match kind {
        ImportKind::Movie(id) => import_movie(db, paths, src_dir, id, release_title).await,
        ImportKind::Series(ids) => import_episodes(db, paths, src_dir, &ids, release_title).await,
        ImportKind::Album(id) => import_album(db, paths, src_dir, id, release_title).await,
    }
}

async fn import_movie(
    db: &Db,
    paths: &PathsSettings,
    src_dir: &Path,
    movie_id: i64,
    release_title: &str,
) -> Result<ImportOutcome> {
    let movie = media::get_movie(db, movie_id).await?;
    let files = collect_files(src_dir, "movie").await;
    let Some(main) = files.first() else {
        bail!("no video files in {}", src_dir.display());
    };
    let root = if movie.path.is_empty() {
        naming::movie_folder(Path::new(&paths.movies_root), &movie.title, movie.year)
    } else {
        PathBuf::from(&movie.path)
    };
    let quality = {
        let q = quality_of(
            &main
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
        );
        if q.is_empty() {
            quality_of(release_title)
        } else {
            q
        }
    };
    let ext = naming::ext_of(main);
    let dest = root.join(naming::movie_file(&movie.title, movie.year, &quality, &ext));
    place_file(main, &dest, &paths.import_mode).await?;

    // Pick up sidecar subtitles next to the feature file.
    let mut imported_paths = vec![dest.to_string_lossy().into_owned()];
    for f in &files[1..] {
        // Secondary video files (extras) get placed too, keeping their name.
        let side = root.join(
            naming::sanitize(
                &f.file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            ) + "."
                + &naming::ext_of(f),
        );
        if place_file(f, &side, &paths.import_mode).await.is_ok() {
            imported_paths.push(side.to_string_lossy().into_owned());
        }
    }
    for sub in collect_subs(src_dir, &movie.title) {
        if let Some(d) = &sub {
            imported_paths.push(d.to_string_lossy().into_owned());
        }
    }

    let size = dest.metadata().map(|m| m.len() as i64).unwrap_or(0);
    let _file_id = media::link_file(
        db,
        &dest.to_string_lossy(),
        size,
        &quality,
        Some(movie_id),
        None,
        None,
    )
    .await?;
    sqlx::query("UPDATE movies SET status='imported', path=? WHERE id=?")
        .bind(root.to_string_lossy().into_owned())
        .bind(movie_id)
        .execute(db.pool())
        .await?;
    Ok(ImportOutcome {
        imported: imported_paths.len(),
        skipped: 0,
        files: imported_paths,
    })
}

fn collect_subs(src_dir: &Path, _title: &str) -> Vec<Option<PathBuf>> {
    // Subtitle handling is a TODO: return none rather than misplacing them.
    let _ = src_dir;
    Vec::new()
}

async fn import_episodes(
    db: &Db,
    paths: &PathsSettings,
    src_dir: &Path,
    episode_ids: &[i64],
    release_title: &str,
) -> Result<ImportOutcome> {
    if episode_ids.is_empty() {
        bail!("no target episodes recorded for this download");
    }
    let mut targets = Vec::new();
    for id in episode_ids {
        let ep = sqlx::query_as::<_, media::Episode>("SELECT * FROM episodes WHERE id = ?")
            .bind(*id)
            .fetch_one(db.pool())
            .await?;
        targets.push(ep);
    }
    let series = media::get_series(db, targets[0].series_id).await?;
    let series_root = if series.path.is_empty() {
        naming::series_folder(Path::new(&paths.series_root), &series.title)
    } else {
        PathBuf::from(&series.path)
    };

    let files = collect_files(src_dir, "series").await;
    if files.is_empty() {
        bail!("no video files in {}", src_dir.display());
    }

    let mut imported_paths = Vec::new();
    let mut skipped = 0usize;

    // Try to match every video file to a wanted episode.
    let mut unmatched_files: Vec<PathBuf> = Vec::new();
    let mut done_eps: std::collections::HashSet<i64> = std::collections::HashSet::new();
    for file in &files {
        let fname = file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let parsed = parse::parse_release(&fname);
        // A file can cover several episodes (S01E01E02); collect all of them.
        let matched: Vec<&media::Episode> = targets
            .iter()
            .filter(|ep| {
                !done_eps.contains(&ep.id)
                    && parse::matches_episode(
                        &parsed,
                        ep.season_number as u32,
                        &[ep.episode_number as u32],
                    )
            })
            .collect();
        if matched.is_empty() {
            unmatched_files.push(file.clone());
        } else {
            for ep in &matched {
                done_eps.insert(ep.id);
            }
            let ctx = EpImport {
                db,
                paths,
                series: &series,
                series_root: &series_root,
                release_title,
            };
            ctx.import_for(&matched, file, &mut imported_paths).await?;
        }
    }

    // A single-file download for several episodes (or an unparseable name):
    // attach it to the first remaining target.
    if unmatched_files.len() == 1 {
        let file = unmatched_files.remove(0);
        if let Some(ep) = targets.iter().find(|e| !done_eps.contains(&e.id)) {
            done_eps.insert(ep.id);
            let ctx = EpImport {
                db,
                paths,
                series: &series,
                series_root: &series_root,
                release_title,
            };
            ctx.import_for(&[ep], &file, &mut imported_paths).await?;
        }
    } else {
        skipped += unmatched_files.len();
    }
    if imported_paths.is_empty() {
        bail!(
            "could not match any files in {} to wanted episodes",
            src_dir.display()
        );
    }
    Ok(ImportOutcome {
        imported: imported_paths.len(),
        skipped,
        files: imported_paths,
    })
}

/// Shared context for placing episode files: one instance per season pack.
struct EpImport<'a> {
    db: &'a Db,
    paths: &'a PathsSettings,
    series: &'a media::Series,
    series_root: &'a Path,
    release_title: &'a str,
}

impl EpImport<'_> {
    /// Place `file` once and link it to every episode it covers. A
    /// multi-episode file gets a multi-episode name (S01E01E02); all covered
    /// episodes share the file_id.
    async fn import_for(
        &self,
        eps: &[&media::Episode],
        file: &Path,
        out: &mut Vec<String>,
    ) -> Result<()> {
        let Some(&ep) = eps.first() else {
            return Ok(());
        };
        let ep_numbers: Vec<u32> = eps.iter().map(|e| e.episode_number as u32).collect();
        let quality = {
            let q = quality_of(
                &file
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            );
            if q.is_empty() {
                quality_of(self.release_title)
            } else {
                q
            }
        };
        let ext = naming::ext_of(file);
        let season_dir = self.series_root.join(naming::season_folder(
            &self.series.title,
            ep.season_number as u32,
        ));
        let dest = season_dir.join(naming::episode_file(
            &self.series.title,
            ep.season_number as u32,
            &ep_numbers,
            if eps.len() > 1 { "" } else { &ep.title },
            &quality,
            &ext,
        ));
        place_file(file, &dest, &self.paths.import_mode).await?;
        let size = dest.metadata().map(|m| m.len() as i64).unwrap_or(0);
        let file_id = media::link_file(
            self.db,
            &dest.to_string_lossy(),
            size,
            &quality,
            None,
            Some(ep.id),
            None,
        )
        .await?;
        for e in eps {
            sqlx::query("UPDATE episodes SET status='imported', file_id=? WHERE id=?")
                .bind(file_id)
                .bind(e.id)
                .execute(self.db.pool())
                .await?;
        }
        if self.series.path.is_empty() {
            sqlx::query("UPDATE series SET path=? WHERE id=?")
                .bind(self.series_root.to_string_lossy().into_owned())
                .bind(self.series.id)
                .execute(self.db.pool())
                .await?;
        }
        out.push(dest.to_string_lossy().into_owned());
        Ok(())
    }
}

async fn import_album(
    db: &Db,
    paths: &PathsSettings,
    src_dir: &Path,
    album_id: i64,
    release_title: &str,
) -> Result<ImportOutcome> {
    let album = sqlx::query_as::<_, media::Album>("SELECT * FROM albums WHERE id=?")
        .bind(album_id)
        .fetch_one(db.pool())
        .await?;
    let artist = media::get_artist(db, album.artist_id).await?;
    let files = collect_files(src_dir, "album").await;
    if files.is_empty() {
        bail!("no audio files in {}", src_dir.display());
    }
    let year = album
        .release_date
        .as_deref()
        .and_then(|d| d.get(..4))
        .and_then(|y| y.parse::<i64>().ok());
    let dest_dir = naming::album_folder(
        Path::new(&paths.music_root),
        &artist.name,
        &album.title,
        year,
    );
    let mut imported_paths = Vec::new();
    let mut total = 0i64;
    for f in &files {
        let dest = dest_dir.join(
            f.file_name()
                .map(|n| naming::sanitize(&n.to_string_lossy()))
                .unwrap_or_else(|| "track".into()),
        );
        // Sanitize killed the extension when the filename had dots in odd
        // places, rebuild it deterministically.
        let dest = if naming::ext_of(&dest).is_empty() {
            dest.with_extension(naming::ext_of(f))
        } else {
            dest
        };
        place_file(f, &dest, &paths.import_mode).await?;
        total += dest.metadata().map(|m| m.len() as i64).unwrap_or(0);
        imported_paths.push(dest.to_string_lossy().into_owned());
    }
    let file_id = media::link_file(
        db,
        &dest_dir.to_string_lossy(),
        total,
        &quality_of(release_title),
        None,
        None,
        Some(album_id),
    )
    .await?;
    sqlx::query("UPDATE albums SET status='imported', file_id=? WHERE id=?")
        .bind(file_id)
        .bind(album_id)
        .execute(db.pool())
        .await?;
    Ok(ImportOutcome {
        imported: imported_paths.len(),
        skipped: 0,
        files: imported_paths,
    })
}
