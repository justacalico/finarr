//! MusicBrainz client — artists + release groups. Free, no key.
//! MusicBrainz rate limits to ~1 req/sec; all calls are serialized
//! through a shared limiter.

use std::sync::LazyLock;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::Deserialize;
use tokio::sync::Mutex;

use super::{AlbumResult, ArtistResult};

const BASE: &str = "https://musicbrainz.org/ws/2";
const CAA: &str = "https://coverartarchive.org";

static LIMITER: LazyLock<Mutex<Instant>> = LazyLock::new(|| Mutex::new(Instant::now()));

async fn throttle() {
    let mut last = LIMITER.lock().await;
    let elapsed = last.elapsed();
    if elapsed < Duration::from_millis(1100) {
        tokio::time::sleep(Duration::from_millis(1100) - elapsed).await;
    }
    *last = Instant::now();
}

#[derive(Debug, Deserialize)]
struct ArtistSearch {
    artists: Vec<Artist>,
}

#[derive(Debug, Deserialize)]
struct Artist {
    id: String,
    name: String,
    #[serde(rename = "sort-name")]
    sort_name: Option<String>,
    disambiguation: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReleaseGroupSearch {
    #[serde(rename = "release-groups", default)]
    release_groups: Vec<ReleaseGroup>,
}

#[derive(Debug, Deserialize)]
struct ReleaseGroup {
    id: String,
    title: String,
    #[serde(rename = "primary-type")]
    primary_type: Option<String>,
    #[serde(rename = "first-release-date")]
    first_release: Option<String>,
}

pub async fn search_artists(http: &reqwest::Client, query: &str) -> Result<Vec<ArtistResult>> {
    throttle().await;
    let resp: ArtistSearch = http
        .get(format!("{BASE}/artist/"))
        .query(&[("query", format!("artist:{query}")), ("fmt", "json".into())])
        .send()
        .await
        .context("musicbrainz artist search")?
        .error_for_status()
        .context("musicbrainz status")?
        .json()
        .await
        .context("musicbrainz parse")?;
    Ok(resp
        .artists
        .into_iter()
        .map(|a| ArtistResult {
            mbid: a.id.clone(),
            name: a.name,
            sort_name: a.sort_name.unwrap_or_default(),
            overview: a.disambiguation.unwrap_or_default(),
            // No artist images in MusicBrainz; leave empty and let the UI
            // render a monogram.
            image_url: None,
        })
        .collect())
}

/// Release groups for an artist (albums, EPs, singles...).
pub async fn albums(http: &reqwest::Client, artist_mbid: &str) -> Result<Vec<AlbumResult>> {
    throttle().await;
    let resp: ReleaseGroupSearch = http
        .get(format!("{BASE}/release-group/"))
        .query(&[
            ("artist", artist_mbid.to_string()),
            ("fmt", "json".into()),
            ("limit", "100".into()),
            ("type", "album|ep|single|compilation|live".into()),
        ])
        .send()
        .await
        .context("musicbrainz albums request")?
        .error_for_status()
        .context("musicbrainz albums status")?
        .json()
        .await
        .context("musicbrainz albums parse")?;
    let mut out: Vec<AlbumResult> = resp
        .release_groups
        .into_iter()
        .map(|rg| AlbumResult {
            mbid: Some(rg.id.clone()),
            title: rg.title,
            release_date: rg.first_release,
            album_type: rg.primary_type.unwrap_or_else(|| "album".into()),
        })
        .collect();
    out.sort_by(|a, b| a.release_date.cmp(&b.release_date));
    Ok(out)
}

/// Cover-art URL for a release group (the archive 302s to the image).
pub fn release_group_art(mbid: &str) -> String {
    format!("{CAA}/release-group/{mbid}/front-250")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_art_url() {
        assert_eq!(
            release_group_art("abc"),
            "https://coverartarchive.org/release-group/abc/front-250"
        );
    }
}
