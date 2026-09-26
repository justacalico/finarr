//! Torznab client, works with Jackett, Prowlarr and any Newznab-style
//! indexer endpoint.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ReleaseResult {
    pub title: String,
    pub guid: String,
    /// Magnet or .torrent URL the engine/client can consume.
    pub download_url: String,
    pub info_hash: Option<String>,
    pub size_bytes: u64,
    pub seeders: u32,
    pub peers: u32,
    pub grabs: u32,
    pub published: String,
    pub categories: Vec<u32>,
    /// Set by the caller: which indexer produced this result.
    pub indexer: String,
    /// Score assigned locally (higher = better pick).
    pub score: i64,
}

#[derive(Debug, Clone, Deserialize)]
struct Rss {
    channel: Option<RssChannel>,
}

#[derive(Debug, Clone, Deserialize)]
struct RssChannel {
    #[serde(default)]
    item: Vec<RssItem>,
}

#[derive(Debug, Clone, Deserialize)]
struct RssItem {
    title: Option<String>,
    link: Option<String>,
    guid: Option<RssGuid>,
    #[serde(rename = "pubDate")]
    pub_date: Option<String>,
    enclosure: Option<RssEnclosure>,
    #[serde(rename = "attr", default, alias = "torznab:attr")]
    attrs: Vec<TorznabAttr>,
    /// Non-namespaced `category` elements.
    #[serde(default)]
    #[allow(dead_code)]
    category: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RssGuid {
    #[serde(rename = "$value")]
    value: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RssEnclosure {
    #[serde(rename = "@url")]
    url: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TorznabAttr {
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@value")]
    value: String,
}

#[derive(Debug, Clone, Deserialize)]
struct TorznabCaps {
    #[serde(default)]
    categories: TorznabCategories,
    #[serde(default)]
    searching: Searching,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TorznabCategories {
    #[serde(default, rename = "category")]
    categories: Vec<TorznabCategory>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TorznabCategory {
    #[serde(rename = "@id")]
    pub id: String,
    #[serde(rename = "@name")]
    pub name: String,
    #[serde(default, rename = "subcat")]
    pub subcats: Vec<TorznabCategory>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct Searching {
    search: Option<SearchCaps>,
    #[serde(rename = "tv-search")]
    tv_search: Option<SearchCaps>,
    #[serde(rename = "movie-search")]
    movie_search: Option<SearchCaps>,
    #[serde(rename = "music-search")]
    music_search: Option<SearchCaps>,
    #[serde(rename = "audio-search")]
    audio_search: Option<SearchCaps>,
}

#[derive(Debug, Clone, Deserialize)]
struct SearchCaps {
    #[serde(rename = "@available")]
    available: String,
    #[serde(rename = "@supportedParams", default)]
    #[allow(dead_code)]
    supported_params: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapsInfo {
    pub movie_search: bool,
    pub tv_search: bool,
    pub music_search: bool,
    pub text_search: bool,
    pub categories: Vec<String>,
}

pub struct TorznabClient {
    url: String,
    api_key: String,
    http: reqwest::Client,
}

#[derive(Debug, Clone, Default)]
pub struct SearchQuery {
    /// search | movie | tvsearch | music | audio | book
    pub kind: String,
    pub query: String,
    pub season: Option<u32>,
    pub episode: Option<u32>,
    pub imdb_id: Option<String>,
    pub categories: Vec<u32>,
}

impl TorznabClient {
    pub fn new(url: &str, api_key: &str, http: reqwest::Client) -> Self {
        Self {
            url: url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            http,
        }
    }

    /// Jackett and friends accept the apikey either as ?apikey= or ?api_key=.
    fn endpoint(&self, extra: &[(&str, String)]) -> String {
        let mut params: Vec<(String, String)> = vec![("apikey".into(), self.api_key.clone())];
        for (k, v) in extra {
            params.push((k.to_string(), v.clone()));
        }
        let qs = serde_urlencoded::to_string(params).unwrap_or_default();
        if self.url.contains('?') {
            format!("{}&{qs}", self.url)
        } else {
            format!("{}?{qs}", self.url)
        }
    }

    async fn get(&self, extra: &[(&str, String)]) -> Result<String> {
        let url = self.endpoint(extra);
        let resp = self
            .http
            .get(&url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .with_context(|| format!("torznab request failed: {}", self.url))?;
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if !status.is_success() {
            bail!(
                "torznab returned {status}: {}",
                &body[..body.len().min(300)]
            );
        }
        if body.contains("<error") {
            bail!("torznab error: {}", &body[..body.len().min(300)]);
        }
        Ok(body)
    }

    /// Fetch capabilities; used by the "test" button in settings.
    pub async fn caps(&self) -> Result<CapsInfo> {
        let body = self.get(&[("t", "caps".to_string())]).await?;
        let caps: TorznabCaps = quick_xml::de::from_str(&body).context("parse torznab caps")?;
        let mut categories = Vec::new();
        for c in &caps.categories.categories {
            categories.push(format!("{}: {}", c.id, c.name));
            for s in &c.subcats {
                categories.push(format!("{}: {}", s.id, s.name));
            }
        }
        let avail =
            |s: &Option<SearchCaps>| s.as_ref().map(|s| s.available == "yes").unwrap_or(false);
        Ok(CapsInfo {
            movie_search: avail(&caps.searching.movie_search),
            tv_search: avail(&caps.searching.tv_search),
            music_search: avail(&caps.searching.music_search)
                || avail(&caps.searching.audio_search),
            text_search: avail(&caps.searching.search),
            categories,
        })
    }

    pub async fn search(&self, q: &SearchQuery) -> Result<Vec<ReleaseResult>> {
        let mut params: Vec<(&str, String)> = vec![
            ("t", q.kind.clone()),
            ("q", q.query.clone()),
            ("limit", "100".into()),
        ];
        if let Some(s) = q.season {
            params.push(("season", s.to_string()));
        }
        if let Some(e) = q.episode {
            params.push(("ep", e.to_string()));
        }
        if let Some(imdb) = &q.imdb_id {
            params.push(("imdbid", imdb.trim_start_matches("tt").to_string()));
        }
        if !q.categories.is_empty() {
            params.push((
                "cat",
                q.categories
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            ));
        }
        let body = self.get(&params).await?;
        parse_rss(&body)
    }
}

fn attr<'a>(attrs: &'a [TorznabAttr], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|a| a.name.eq_ignore_ascii_case(name))
        .map(|a| a.value.as_str())
}

pub fn parse_rss(body: &str) -> Result<Vec<ReleaseResult>> {
    let rss: Rss = quick_xml::de::from_str(body).context("parse torznab rss")?;
    let items = rss.channel.map(|c| c.item).unwrap_or_default();
    Ok(items
        .into_iter()
        .filter_map(|it| {
            let attrs = &it.attrs;
            let download_url = it
                .enclosure
                .map(|e| e.url)
                .or_else(|| attr(attrs, "magneturl").map(str::to_string))
                .or_else(|| attr(attrs, "downloadurl").map(str::to_string))
                .or(it.link.clone())?;
            let size = attr(attrs, "size")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let seeders = attr(attrs, "seeders")
                .or_else(|| attr(attrs, "seed"))
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let peers = attr(attrs, "peers")
                .and_then(|v| v.parse().ok())
                .unwrap_or(seeders);
            let grabs = attr(attrs, "grabs")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            let info_hash = attr(attrs, "infohash")
                .or_else(|| attr(attrs, "info_hash"))
                .map(|v| v.to_lowercase());
            let categories = attrs
                .iter()
                .filter(|a| a.name.eq_ignore_ascii_case("category"))
                .filter_map(|a| a.value.parse::<u32>().ok())
                .collect::<Vec<_>>();
            let guid = it
                .guid
                .map(|g| g.value)
                .or_else(|| it.link.clone())
                .unwrap_or_default();
            Some(ReleaseResult {
                title: it.title.unwrap_or_else(|| "unknown".into()),
                guid,
                download_url,
                info_hash,
                size_bytes: size,
                seeders,
                peers,
                grabs,
                published: it.pub_date.unwrap_or_default(),
                categories,
                indexer: String::new(),
                score: 0,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0"?>
<rss version="2.0" xmlns:torznab="http://torznab.com/schemas/2015/feed">
  <channel>
    <item>
      <title>Test.Show.S01E02.1080p.WEB-DL.DDP5.1.H.264-GRP</title>
      <guid isPermaLink="true">https://tracker.example/1</guid>
      <pubDate>Mon, 01 Jan 2024 00:00:00 +0000</pubDate>
      <enclosure url="magnet:?xt=urn:btih:abc123&amp;dn=test" length="1500000000" type="application/x-bittorrent" />
      <torznab:attr name="size" value="1500000000" />
      <torznab:attr name="seeders" value="42" />
      <torznab:attr name="peers" value="50" />
      <torznab:attr name="infohash" value="ABC123" />
      <torznab:attr name="category" value="5030" />
    </item>
    <item>
      <title>Bad.Item.With.No.Link</title>
      <pubDate>Mon, 01 Jan 2024 00:00:00 +0000</pubDate>
    </item>
  </channel>
</rss>"#;

    #[test]
    fn parses_torznab_items() {
        let items = parse_rss(SAMPLE).unwrap();
        assert_eq!(items.len(), 1);
        let r = &items[0];
        assert_eq!(r.title, "Test.Show.S01E02.1080p.WEB-DL.DDP5.1.H.264-GRP");
        assert!(r.download_url.starts_with("magnet:"));
        assert_eq!(r.seeders, 42);
        assert_eq!(r.info_hash.as_deref(), Some("abc123"));
        assert_eq!(r.categories, vec![5030]);
    }
}
