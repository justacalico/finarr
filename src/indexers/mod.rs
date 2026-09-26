//! Indexer management and aggregate search across enabled indexers.

pub mod torznab;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::db::Db;
use crate::media::parse;
use torznab::{ReleaseResult, SearchQuery, TorznabClient};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Indexer {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub api_key: String,
    pub enabled: i64,
    pub priority: i64,
    /// JSON array of torznab category ids to include.
    pub categories: String,
    pub created_at: String,
}

impl Indexer {
    pub fn category_list(&self) -> Vec<u32> {
        serde_json::from_str(&self.categories).unwrap_or_default()
    }
}

pub async fn list(db: &Db) -> Result<Vec<Indexer>> {
    Ok(
        sqlx::query_as::<_, Indexer>("SELECT * FROM indexers ORDER BY priority, name")
            .fetch_all(db.pool())
            .await?,
    )
}

pub async fn get(db: &Db, id: i64) -> Result<Option<Indexer>> {
    Ok(
        sqlx::query_as::<_, Indexer>("SELECT * FROM indexers WHERE id = ?")
            .bind(id)
            .fetch_optional(db.pool())
            .await?,
    )
}

/// Everything needed to create or update an indexer row.
pub struct IndexerConfig<'a> {
    pub name: &'a str,
    pub url: &'a str,
    pub api_key: &'a str,
    pub enabled: bool,
    pub categories: &'a [u32],
    pub priority: i64,
}

pub async fn create(db: &Db, cfg: &IndexerConfig<'_>) -> Result<Indexer> {
    let cats = serde_json::to_string(cfg.categories)?;
    let row = sqlx::query(
        "INSERT INTO indexers (name, url, api_key, categories, priority)
         VALUES (?, ?, ?, ?, ?) RETURNING id",
    )
    .bind(cfg.name)
    .bind(cfg.url)
    .bind(cfg.api_key)
    .bind(&cats)
    .bind(cfg.priority)
    .fetch_one(db.pool())
    .await?;
    Ok(Indexer {
        id: row.get("id"),
        name: cfg.name.into(),
        url: cfg.url.into(),
        api_key: cfg.api_key.into(),
        enabled: 1,
        priority: cfg.priority,
        categories: cats,
        created_at: String::new(),
    })
}

pub async fn update(db: &Db, id: i64, cfg: &IndexerConfig<'_>) -> Result<()> {
    sqlx::query(
        "UPDATE indexers SET name=?, url=?, api_key=?, enabled=?, categories=?, priority=?
         WHERE id=?",
    )
    .bind(cfg.name)
    .bind(cfg.url)
    .bind(cfg.api_key)
    .bind(cfg.enabled as i64)
    .bind(serde_json::to_string(cfg.categories)?)
    .bind(cfg.priority)
    .bind(id)
    .execute(db.pool())
    .await?;
    Ok(())
}

pub async fn delete(db: &Db, id: i64) -> Result<()> {
    sqlx::query("DELETE FROM indexers WHERE id = ?")
        .bind(id)
        .execute(db.pool())
        .await?;
    Ok(())
}

/// Search every enabled indexer in parallel and merge/dedupe results.
/// Individual indexer failures are logged and skipped, never fatal.
pub async fn search_all(db: &Db, http: reqwest::Client, q: &SearchQuery) -> Vec<ReleaseResult> {
    let indexers = match list(db).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("list indexers failed: {e:?}");
            return Vec::new();
        }
    };
    let mut tasks = Vec::new();
    for idx in indexers.iter().filter(|i| i.enabled != 0) {
        let client = TorznabClient::new(&idx.url, &idx.api_key, http.clone());
        let mut qq = q.clone();
        if !idx.category_list().is_empty() && qq.categories.is_empty() {
            qq.categories = idx.category_list();
        }
        let name = idx.name.clone();
        tasks.push(async move {
            let res =
                tokio::time::timeout(std::time::Duration::from_secs(45), client.search(&qq)).await;
            (name, res)
        });
    }
    let mut out: Vec<ReleaseResult> = Vec::new();
    for (name, res) in futures::future::join_all(tasks).await {
        match res {
            Ok(Ok(items)) => {
                for mut it in items {
                    it.indexer = name.clone();
                    out.push(it);
                }
            }
            Ok(Err(e)) => tracing::warn!("indexer {name} search failed: {e:#}"),
            Err(_) => tracing::warn!("indexer {name} search timed out"),
        }
    }
    dedupe(&mut out);
    out
}

fn dedupe(items: &mut Vec<ReleaseResult>) {
    let mut seen = std::collections::HashSet::new();
    items.retain(|r| {
        let key = r
            .info_hash
            .clone()
            .unwrap_or_else(|| format!("{}:{}", r.guid, r.indexer));
        seen.insert(key)
    });
}

/// Score a release against what we want: prefer correct episodes, higher
/// quality and healthier swarms.
pub fn score_release(r: &ReleaseResult, want_season: Option<u32>, want_episodes: &[u32]) -> i64 {
    let parsed = parse::parse_release(&r.title);
    let mut score = parse::quality_rank(&parsed);
    if let (Some(ws), Some(ps)) = (want_season, parsed.season) {
        if ws == ps {
            score += 15;
        } else {
            score -= 40;
        }
    } else if want_season.is_some() && parsed.season.is_some() {
        score -= 40;
    }
    if !want_episodes.is_empty() {
        if parse::matches_episode(&parsed, want_season.unwrap_or(0), want_episodes) {
            score += 25;
        } else if parsed.season_pack && parsed.season == want_season {
            score += 20;
        }
    }
    // Mild preference for healthy swarms.
    score += (r.seeders as i64).min(100) / 4;
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rel(title: &str, seeders: u32) -> ReleaseResult {
        ReleaseResult {
            title: title.into(),
            guid: title.into(),
            download_url: "magnet:?xt=x".into(),
            info_hash: Some(title.into()),
            size_bytes: 1_000,
            seeders,
            peers: 0,
            grabs: 0,
            published: String::new(),
            categories: vec![],
            indexer: "t".into(),
            score: 0,
        }
    }

    #[test]
    fn correct_episode_wins_over_wrong() {
        let want = score_release(&rel("Show.S01E05.1080p.WEB-DL-GRP", 10), Some(1), &[5]);
        let wrong = score_release(&rel("Show.S01E06.1080p.WEB-DL-GRP", 10), Some(1), &[5]);
        assert!(want > wrong, "{want} !> {wrong}");
        assert!(want > 0);
    }

    #[test]
    fn season_pack_scores_for_all_episodes() {
        let pack = score_release(&rel("Show.S01.COMPLETE.1080p-GRP", 10), Some(1), &[1, 2, 3]);
        assert!(pack > 0);
    }

    #[test]
    fn quality_is_ranked() {
        let hd = score_release(&rel("Movie.2024.2160p.WEB-DL-GRP", 10), None, &[]);
        let sd = score_release(&rel("Movie.2024.480p.WEB-DL-GRP", 10), None, &[]);
        assert!(hd > sd);
    }

    #[test]
    fn wrong_season_penalized() {
        let s2 = score_release(&rel("Show.S02E01.1080p-GRP", 10), Some(1), &[1]);
        let s1 = score_release(&rel("Show.S01E01.1080p-GRP", 10), Some(1), &[1]);
        assert!(s1 > s2);
    }

    #[test]
    fn healthier_swarm_scores_higher() {
        let healthy = score_release(&rel("Movie.2024.1080p-GRP", 100), None, &[]);
        let dead = score_release(&rel("Movie.2024.1080p-GRP", 1), None, &[]);
        assert!(healthy > dead);
    }
}
