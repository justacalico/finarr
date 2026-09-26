//! TMDB v3 API client — movies (and optionally series) metadata.
//! Requires an API key configured in Settings > Metadata.

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use super::MovieResult;

const BASE: &str = "https://api.themoviedb.org/3";
const IMG: &str = "https://image.tmdb.org/t/p";

#[derive(Debug, Deserialize)]
struct SearchResponse {
    results: Vec<SearchMovie>,
}

#[derive(Debug, Deserialize)]
struct SearchMovie {
    id: i64,
    title: Option<String>,
    original_title: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    release_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MovieDetails {
    id: i64,
    title: Option<String>,
    original_title: Option<String>,
    overview: Option<String>,
    poster_path: Option<String>,
    backdrop_path: Option<String>,
    release_date: Option<String>,
    runtime: Option<i64>,
    #[serde(default)]
    genres: Vec<Genre>,
    imdb_id: Option<String>,
    release_dates: Option<ReleaseDates>,
}

#[derive(Debug, Deserialize)]
struct Genre {
    name: String,
}

#[derive(Debug, Deserialize)]
struct ReleaseDates {
    results: Vec<ReleaseCountry>,
}

#[derive(Debug, Deserialize)]
struct ReleaseCountry {
    iso_3166_1: String,
    release_dates: Vec<ReleaseDate>,
}

#[derive(Debug, Deserialize)]
struct ReleaseDate {
    release_date: String,
    #[serde(rename = "type")]
    kind: i64,
}

fn poster(p: &Option<String>) -> Option<String> {
    p.as_ref().map(|p| format!("{IMG}/w500{p}"))
}

fn backdrop(p: &Option<String>) -> Option<String> {
    p.as_ref().map(|p| format!("{IMG}/w1280{p}"))
}

fn year_of(date: &Option<String>) -> Option<i64> {
    date.as_ref()?.get(..4)?.parse().ok()
}

impl From<SearchMovie> for MovieResult {
    fn from(m: SearchMovie) -> Self {
        MovieResult {
            tmdb_id: m.id,
            imdb_id: None,
            title: m.title.clone().or(m.original_title.clone()).unwrap_or_default(),
            original_title: m.original_title,
            year: year_of(&m.release_date),
            overview: m.overview.unwrap_or_default(),
            poster_url: poster(&m.poster_path),
            backdrop_url: backdrop(&m.backdrop_path),
            runtime_min: None,
            genres: Vec::new(),
            release_date: m.release_date,
            digital_date: None,
        }
    }
}

pub async fn search_movie(http: &reqwest::Client, api_key: &str, query: &str) -> Result<Vec<MovieResult>> {
    let resp: SearchResponse = http
        .get(format!("{BASE}/search/movie"))
        .query(&[
            ("api_key", api_key),
            ("query", query),
            ("include_adult", "false"),
        ])
        .send()
        .await
        .context("tmdb search request")?
        .error_for_status()
        .context("tmdb search status")?
        .json()
        .await
        .context("tmdb search parse")?;
    Ok(resp.results.into_iter().map(Into::into).collect())
}

/// Digital release date: type 4 in release_dates, preferring US.
fn digital_date(d: &MovieDetails) -> Option<String> {
    let rd = d.release_dates.as_ref()?;
    let pick = |cc: &str| {
        rd.results
            .iter()
            .find(|c| c.iso_3166_1 == cc)
            .and_then(|c| {
                c.release_dates
                    .iter()
                    .find(|r| r.kind == 4)
                    .map(|r| r.release_date.chars().take(10).collect::<String>())
            })
    };
    pick("US")
        .or_else(|| rd.results.iter().find_map(|c| {
            c.release_dates
                .iter()
                .find(|r| r.kind == 4)
                .map(|r| r.release_date.chars().take(10).collect::<String>())
        }))
}

pub async fn movie(http: &reqwest::Client, api_key: &str, tmdb_id: i64) -> Result<MovieResult> {
    let resp = http
        .get(format!("{BASE}/movie/{tmdb_id}"))
        .query(&[
            ("api_key", api_key),
            ("append_to_response", "external_ids,release_dates"),
        ])
        .send()
        .await
        .context("tmdb movie request")?;
    if !resp.status().is_success() {
        bail!("tmdb movie {tmdb_id}: {}", resp.status());
    }
    let d: MovieDetails = resp.json().await.context("tmdb movie parse")?;
    let dd = digital_date(&d);
    Ok(MovieResult {
        tmdb_id: d.id,
        imdb_id: d.imdb_id,
        title: d.title.clone().or(d.original_title.clone()).unwrap_or_default(),
        original_title: d.original_title,
        year: year_of(&d.release_date),
        overview: d.overview.unwrap_or_default(),
        poster_url: poster(&d.poster_path),
        backdrop_url: backdrop(&d.backdrop_path),
        runtime_min: d.runtime,
        genres: d.genres.into_iter().map(|g| g.name).collect(),
        release_date: d.release_date,
        digital_date: dd,
    })
}
