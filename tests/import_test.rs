//! End-to-end import test: add a movie, drop a fake download, import it,
//! verify the file lands in the library with the right name and status.

use std::path::PathBuf;

use finarr::db::Db;
use finarr::media::{self, import};
use finarr::metadata::MovieResult;
use finarr::settings::PathsSettings;

fn dirs() -> (PathBuf, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!("finarr-test-{}", uuid::Uuid::new_v4()));
    let downloads = root.join("downloads");
    let library = root.join("library/movies");
    std::fs::create_dir_all(&downloads).unwrap();
    std::fs::create_dir_all(&library).unwrap();
    (root, downloads, library)
}

#[tokio::test]
async fn movie_import_places_file_and_marks_imported() {
    let (root, downloads, library) = dirs();
    let db_path = root.join("t.db");
    let db = Db::connect(&format!("sqlite:{}?mode=rwc", db_path.display()))
        .await
        .unwrap();

    // Register the movie in the library.
    let m = MovieResult {
        tmdb_id: 1,
        imdb_id: Some("tt0000001".into()),
        title: "Test Film".into(),
        original_title: Some("Test Film".into()),
        year: Some(2024),
        overview: String::new(),
        runtime_min: None,
        genres: vec![],
        poster_url: None,
        backdrop_url: None,
        release_date: None,
        digital_date: None,
    };
    let movie = media::add_movie(&db, &m, library.to_str().unwrap(), true)
        .await
        .unwrap();
    assert_eq!(movie.status, "missing");

    // Fake a completed download.
    let src_dir = downloads.join("Test.Film.2024.1080p.WEB-DL.x264-GRP");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("Test.Film.2024.1080p.WEB-DL.x264-GRP.mkv"), b"video")
        .unwrap();
    std::fs::write(src_dir.join("readme.nfo"), b"info").unwrap();

    let paths = PathsSettings {
        downloads_dir: downloads.to_string_lossy().into_owned(),
        movies_root: library.to_string_lossy().into_owned(),
        series_root: root.join("library/series").to_string_lossy().into_owned(),
        music_root: root.join("library/music").to_string_lossy().into_owned(),
        import_mode: "hardlink".into(),
    };

    let outcome = import::import(
        &db,
        &paths,
        &src_dir,
        import::ImportKind::Movie(movie.id),
        "Test.Film.2024.1080p.WEB-DL.x264-GRP",
    )
    .await
    .unwrap();

    assert_eq!(outcome.imported, 1);
    let dest = library
        .join("Test Film (2024)")
        .join("Test Film (2024) [WEB-DL 1080p].mkv");
    assert!(dest.exists(), "expected {dest:?}");
    let after = media::get_movie(&db, movie.id).await.unwrap();
    assert_eq!(after.status, "imported");
    assert_eq!(after.path, library.join("Test Film (2024)").to_string_lossy());

    std::fs::remove_dir_all(&root).ok();
}

#[tokio::test]
async fn series_import_matches_episode_numbers() {
    let (root, downloads, library_root) = dirs();
    let series_root = root.join("library/series");
    std::fs::create_dir_all(&series_root).unwrap();
    let db_path = root.join("t.db");
    let db = Db::connect(&format!("sqlite:{}?mode=rwc", db_path.display()))
        .await
        .unwrap();

    let s = finarr::metadata::SeriesResult {
        tvmaze_id: 7,
        tvdb_id: Some(7),
        imdb_id: None,
        title: "Some Show".into(),
        overview: String::new(),
        poster_url: None,
        backdrop_url: None,
        year: Some(2020),
        network: None,
        air_time: None,
        status: "continuing".into(),
        seasons: vec![finarr::metadata::SeasonResult {
            number: 1,
            episodes: vec![
                finarr::metadata::EpisodeResult {
                    season: 1,
                    number: 1,
                    title: "Pilot".into(),
                    air_date: None,
                    overview: String::new(),
                    runtime_min: None,
                },
                finarr::metadata::EpisodeResult {
                    season: 1,
                    number: 2,
                    title: "Second".into(),
                    air_date: None,
                    overview: String::new(),
                    runtime_min: None,
                },
            ],
        }],
    };
    let series = media::add_series(&db, &s, series_root.to_str().unwrap(), &[], true)
        .await
        .unwrap();
    let eps = media::series_episodes(&db, series.id).await.unwrap();
    assert_eq!(eps.len(), 2);

    let src_dir = downloads.join("Some.Show.S01E01E02.720p.WEB-DL-GRP");
    std::fs::create_dir_all(&src_dir).unwrap();
    std::fs::write(src_dir.join("Some.Show.S01E01.720p.WEB-DL-GRP.mkv"), b"v1").unwrap();
    std::fs::write(src_dir.join("Some.Show.S01E02.720p.WEB-DL-GRP.mkv"), b"v2").unwrap();

    let paths = PathsSettings {
        downloads_dir: downloads.to_string_lossy().into_owned(),
        movies_root: library_root.to_string_lossy().into_owned(),
        series_root: series_root.to_string_lossy().into_owned(),
        music_root: root.join("library/music").to_string_lossy().into_owned(),
        import_mode: "hardlink".into(),
    };

    let outcome = import::import(
        &db,
        &paths,
        &src_dir,
        import::ImportKind::Series(eps.iter().map(|e| e.id).collect()),
        "Some.Show.S01E01E02.720p.WEB-DL-GRP",
    )
    .await
    .unwrap();

    assert_eq!(outcome.imported, 2, "{outcome:?}");
    let season_dir = series_root.join("Some Show").join("Season 01");
    assert!(season_dir.join("Some Show - S01E01 - Pilot [WEB-DL 720p].mkv").exists());
    assert!(season_dir.join("Some Show - S01E02 - Second [WEB-DL 720p].mkv").exists());

    std::fs::remove_dir_all(&root).ok();
}
