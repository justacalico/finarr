//! Metadata providers. TMDB requires an API key (configured in Settings);
//! TVmaze and MusicBrainz need nothing.

pub mod musicbrainz;
pub mod tmdb;
pub mod tvmaze;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovieResult {
    pub tmdb_id: i64,
    pub imdb_id: Option<String>,
    pub title: String,
    pub original_title: Option<String>,
    pub year: Option<i64>,
    pub overview: String,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub runtime_min: Option<i64>,
    pub genres: Vec<String>,
    pub release_date: Option<String>,
    /// Digital/streaming release when TMDB knows it.
    pub digital_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeriesResult {
    pub tvmaze_id: i64,
    pub tvdb_id: Option<i64>,
    pub imdb_id: Option<String>,
    pub title: String,
    pub overview: String,
    pub poster_url: Option<String>,
    pub backdrop_url: Option<String>,
    pub year: Option<i64>,
    pub network: Option<String>,
    pub air_time: Option<String>,
    /// continuing | ended | upcoming | unknown
    pub status: String,
    pub seasons: Vec<SeasonResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonResult {
    pub number: u32,
    pub episodes: Vec<EpisodeResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodeResult {
    pub season: u32,
    pub number: u32,
    pub title: String,
    pub overview: String,
    pub air_date: Option<String>,
    pub runtime_min: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtistResult {
    pub mbid: String,
    pub name: String,
    pub sort_name: String,
    pub overview: String,
    pub image_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlbumResult {
    pub mbid: Option<String>,
    pub title: String,
    pub release_date: Option<String>,
    /// album | ep | single | compilation | ...
    pub album_type: String,
}

pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(format!(
            "finarr/{} (https://gitlab.com/HttpAnimations/finarr)",
            env!("CARGO_PKG_VERSION")
        ))
        .build()
        .unwrap_or_default()
}
