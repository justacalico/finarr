//! TVmaze client, free series metadata + full episode lists, no key.

use anyhow::{Context, Result};
use serde::Deserialize;

use super::{EpisodeResult, SeasonResult, SeriesResult};

const BASE: &str = "https://api.tvmaze.com";

#[derive(Debug, Deserialize)]
struct SearchItem {
    show: Show,
}

#[derive(Debug, Deserialize)]
struct Show {
    id: i64,
    name: String,
    summary: Option<String>,
    image: Option<Image>,
    premiered: Option<String>,
    network: Option<Network>,
    schedule: Option<Schedule>,
    status: Option<String>,
    externals: Option<Externals>,
    #[serde(rename = "_embedded")]
    embedded: Option<Embedded>,
}

#[derive(Debug, Deserialize)]
struct Image {
    medium: Option<String>,
    original: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Network {
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Schedule {
    time: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Externals {
    tvdb: Option<i64>,
    imdb: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Embedded {
    #[serde(default)]
    episodes: Vec<Episode>,
}

#[derive(Debug, Deserialize)]
struct Episode {
    season: u32,
    number: Option<u32>,
    name: Option<String>,
    summary: Option<String>,
    airdate: Option<String>,
    runtime: Option<i64>,
}

fn strip_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

fn status_of(s: &Option<String>) -> String {
    match s.as_deref().map(str::to_lowercase).as_deref() {
        Some("running") => "continuing".into(),
        Some("ended") => "ended".into(),
        Some("to be determined") | Some("in development") => "upcoming".into(),
        _ => "unknown".into(),
    }
}

fn year_of(date: &Option<String>) -> Option<i64> {
    date.as_ref()?.get(..4)?.parse().ok()
}

fn to_result(s: Show) -> SeriesResult {
    let mut seasons: Vec<SeasonResult> = Vec::new();
    if let Some(eps) = s.embedded.as_ref().map(|e| &e.episodes) {
        for ep in eps {
            if seasons.iter().all(|sn| sn.number != ep.season) {
                seasons.push(SeasonResult {
                    number: ep.season,
                    episodes: Vec::new(),
                });
            }
            if let Some(sn) = seasons.iter_mut().find(|sn| sn.number == ep.season) {
                sn.episodes.push(EpisodeResult {
                    season: ep.season,
                    number: ep.number.unwrap_or(0),
                    title: ep.name.clone().unwrap_or_default(),
                    overview: ep.summary.as_deref().map(strip_html).unwrap_or_default(),
                    air_date: ep.airdate.clone(),
                    runtime_min: ep.runtime,
                });
            }
        }
    }
    seasons.sort_by_key(|s| s.number);
    for s in &mut seasons {
        s.episodes.sort_by_key(|e| e.number);
    }
    let (poster, backdrop) = s
        .image
        .as_ref()
        .map(|i| (i.medium.clone(), i.original.clone()))
        .unwrap_or_default();
    SeriesResult {
        tvmaze_id: s.id,
        tvdb_id: s.externals.as_ref().and_then(|e| e.tvdb),
        imdb_id: s.externals.as_ref().and_then(|e| e.imdb.clone()),
        title: s.name,
        overview: s.summary.as_deref().map(strip_html).unwrap_or_default(),
        poster_url: poster,
        backdrop_url: backdrop,
        year: year_of(&s.premiered),
        network: s.network.and_then(|n| n.name),
        air_time: s.schedule.and_then(|sc| sc.time),
        status: status_of(&s.status),
        seasons,
    }
}

/// Search returns shows without embedded episodes; enough for pickers.
pub async fn search(http: &reqwest::Client, query: &str) -> Result<Vec<SeriesResult>> {
    let items: Vec<SearchItem> = http
        .get(format!("{BASE}/search/shows"))
        .query(&[("q", query)])
        .send()
        .await
        .context("tvmaze search request")?
        .error_for_status()
        .context("tvmaze search status")?
        .json()
        .await
        .context("tvmaze search parse")?;
    Ok(items.into_iter().map(|i| to_result(i.show)).collect())
}

/// Full show with the complete episode list embedded.
pub async fn show(http: &reqwest::Client, tvmaze_id: i64) -> Result<SeriesResult> {
    let s: Show = http
        .get(format!("{BASE}/shows/{tvmaze_id}"))
        .query(&[("embed[]", "episodes")])
        .send()
        .await
        .context("tvmaze show request")?
        .error_for_status()
        .context("tvmaze show status")?
        .json()
        .await
        .context("tvmaze show parse")?;
    Ok(to_result(s))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_html() {
        assert_eq!(strip_html("<p>Hello <b>world</b></p>"), "Hello world");
    }

    #[test]
    fn maps_status() {
        assert_eq!(status_of(&Some("Running".into())), "continuing");
        assert_eq!(status_of(&Some("Ended".into())), "ended");
        assert_eq!(status_of(&None), "unknown");
    }
}
