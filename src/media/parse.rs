//! Release-name parsing: quality, resolution, season/episode numbers,
//! year, release group, everything needed to match indexer results to
//! wanted media.

use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParsedRelease {
    /// e.g. "WEB-DL", "BluRay", "HDTV", "FLAC"
    pub source: Option<String>,
    /// 2160 / 1080 / 720 / 480
    pub resolution: Option<u32>,
    pub season: Option<u32>,
    /// Episode numbers found (multi-episode releases have several).
    pub episodes: Vec<u32>,
    /// True when the release is a full season pack (S01, no episode).
    pub season_pack: bool,
    pub year: Option<u32>,
    pub proper: bool,
    pub repack: bool,
    pub group: Option<String>,
}

static RE_SEASON_EP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[.\s_\-\[\(])s(\d{1,2})e(\d{1,3})(?:-?e?(\d{1,3}))?").unwrap()
});
static RE_XEP: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[.\s_\-\[\(])(\d{1,2})x(\d{1,3})(?:-(\d{1,3}))?").unwrap()
});
static RE_SEASON_PACK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|[.\s_\-\[\(])(?:s(\d{1,2})|season[.\s_]*(\d{1,2}))(?:[.\s_\-\]\)]|$)")
        .unwrap()
});
static RE_YEAR: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[.\s_\-\[\(])(19\d{2}|20[0-4]\d)(?:[.\s_\-\]\)]|$)").unwrap()
});
static RE_RES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(?:^|[.\s_\-\[\]])(2160p|1080p|720p|480p|360p)").unwrap());
static RE_SOURCE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(WEB[.\s_-]?DL|WEBRip|BluRay|BDRip|BRRip|REMUX|HDTV|DVDRip|DVD|TSCAM|CAM|TS|FLAC|MP3|AAC|Vinyl)",
    )
    .unwrap()
});
static RE_GROUP: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"-([A-Za-z0-9_'’]+)(?:\.[a-zA-Z0-9]{2,4})?$").unwrap());

pub fn parse_release(title: &str) -> ParsedRelease {
    let mut out = ParsedRelease::default();

    if let Some(c) = RE_SEASON_EP.captures(title) {
        out.season = c.get(1).and_then(|m| m.as_str().parse().ok());
        if let Some(e) = c.get(2).and_then(|m| m.as_str().parse().ok()) {
            out.episodes.push(e);
        }
        // S01E02-E04 or S01E02E03 style ranges.
        if let Some(last) = c.get(3).and_then(|m| m.as_str().parse::<u32>().ok()) {
            let first = out.episodes.first().copied().unwrap_or(0);
            for e in (first + 1)..=last.min(first + 24) {
                out.episodes.push(e);
            }
        }
    } else if let Some(c) = RE_XEP.captures(title) {
        out.season = c.get(1).and_then(|m| m.as_str().parse().ok());
        if let Some(e) = c.get(2).and_then(|m| m.as_str().parse().ok()) {
            out.episodes.push(e);
        }
        if let Some(last) = c.get(3).and_then(|m| m.as_str().parse::<u32>().ok()) {
            let first = out.episodes.first().copied().unwrap_or(0);
            for e in (first + 1)..=last.min(first + 24) {
                out.episodes.push(e);
            }
        }
    } else if let Some(c) = RE_SEASON_PACK.captures(title) {
        out.season = c
            .get(1)
            .or_else(|| c.get(2))
            .and_then(|m| m.as_str().parse().ok());
        out.season_pack = out.season.is_some();
    }

    if let Some(c) = RE_YEAR.captures(title) {
        out.year = c.get(1).and_then(|m| m.as_str().parse().ok());
    }
    if let Some(c) = RE_RES.captures(title) {
        out.resolution = c
            .get(1)
            .and_then(|m| m.as_str().trim_end_matches('p').parse().ok());
    }
    // A release can carry several source tags ("BluRay REMUX"); keep the
    // most specific one by priority.
    const PRIORITY: &[&str] = &[
        "REMUX", "BluRay", "BDRip", "WEB-DL", "BRRip", "WEBRip", "HDTV", "DVDRip", "FLAC", "Vinyl",
        "MP3", "AAC", "CAM",
    ];
    let mut best: Option<(usize, String)> = None;
    for m in RE_SOURCE.find_iter(title) {
        let norm = normalize_source(m.as_str());
        let rank = PRIORITY.iter().position(|p| *p == norm).unwrap_or(99);
        if best.as_ref().map(|(r, _)| rank < *r).unwrap_or(true) {
            best = Some((rank, norm));
        }
    }
    out.source = best.map(|(_, s)| s);
    let lower = title.to_lowercase();
    out.proper = lower.contains("proper") || lower.contains("real.");
    out.repack = lower.contains("repack");
    if let Some(c) = RE_GROUP.captures(title) {
        out.group = Some(c[1].to_string());
    }
    out
}

fn normalize_source(s: &str) -> String {
    let squashed = s.replace(['.', ' ', '_', '-'], "").to_lowercase();
    match squashed.as_str() {
        "webdl" => "WEB-DL".into(),
        "webrip" => "WEBRip".into(),
        "bluray" => "BluRay".into(),
        "bdrip" => "BDRip".into(),
        "brrip" => "BRRip".into(),
        "remux" => "REMUX".into(),
        "hdtv" => "HDTV".into(),
        "dvdrip" | "dvd" => "DVDRip".into(),
        "cam" | "tscam" | "ts" => "CAM".into(),
        "flac" => "FLAC".into(),
        "mp3" => "MP3".into(),
        "aac" => "AAC".into(),
        "vinyl" => "Vinyl".into(),
        other => other.to_string(),
    }
}

/// A rough quality rank: higher is better. Used when no explicit
/// preference is configured, picks the best plausible release.
pub fn quality_rank(p: &ParsedRelease) -> i64 {
    let mut score = 0i64;
    score += match p.resolution {
        Some(2160) => 40,
        Some(1080) => 30,
        Some(720) => 20,
        Some(480) => 5,
        _ => 10,
    };
    score += match p.source.as_deref() {
        Some("REMUX") => 8,
        Some("BluRay") => 7,
        Some("BDRip") => 6,
        Some("WEB-DL") => 5,
        Some("BRRip") => 4,
        Some("WEBRip") => 3,
        Some("HDTV") => 2,
        Some("DVDRip") => 1,
        Some("FLAC") => 7,
        Some("MP3") | Some("AAC") | Some("Vinyl") => 3,
        Some("CAM") => -50,
        _ => 0,
    };
    if p.proper || p.repack {
        score += 3;
    }
    score
}

/// Does this release plausibly match the requested season/episode(s)?
pub fn matches_episode(p: &ParsedRelease, season: u32, episodes: &[u32]) -> bool {
    if p.season != Some(season) {
        return false;
    }
    if p.season_pack {
        return true;
    }
    episodes.iter().all(|e| p.episodes.contains(e)) && !p.episodes.is_empty()
}

/// Lowercase alphanumeric-only form, for fuzzy title containment checks.
fn normalize_title(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Does the release title plausibly refer to `want`? Loose substring match
/// after normalization: "Some.Show.S01E01..." contains "someshow".
pub fn titles_match(want: &str, release_title: &str) -> bool {
    let w = normalize_title(want);
    !w.is_empty() && normalize_title(release_title).contains(&w)
}

/// Hard gate for auto-grabbing TV results. A release must actually cover the
/// wanted episodes (or be the matching season pack) to be eligible at all.
pub fn eligible_tv(p: &ParsedRelease, season: u32, episodes: &[u32]) -> bool {
    if p.season != Some(season) {
        return false;
    }
    if !episodes.is_empty() {
        return matches_episode(p, season, episodes) || (p.season_pack && p.episodes.is_empty());
    }
    p.season_pack || p.episodes.len() >= 2
}

/// Hard gate for auto-grabbing movie results: fuzzy title match, and if the
/// release carries a year it must agree with the wanted one (give or take a
/// year for staggered festival/release-year mismatches).
pub fn eligible_movie(
    p: &ParsedRelease,
    release_title: &str,
    want_title: &str,
    want_year: Option<i64>,
) -> bool {
    if !titles_match(want_title, release_title) {
        return false;
    }
    if let (Some(wy), Some(py)) = (want_year, p.year.map(|y| y as i64)) {
        if (wy - py).abs() > 1 {
            return false;
        }
    }
    p.season.is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_tv() {
        let p = parse_release("The.Great.Show.S02E05.1080p.WEB-DL.DDP5.1.H.264-GRP");
        assert_eq!(p.season, Some(2));
        assert_eq!(p.episodes, vec![5]);
        assert_eq!(p.resolution, Some(1080));
        assert_eq!(p.source.as_deref(), Some("WEB-DL"));
        assert_eq!(p.group.as_deref(), Some("GRP"));
    }

    #[test]
    fn parses_multi_episode() {
        let p = parse_release("Show.S01E01E02.720p.HDTV.x264-ABC");
        assert_eq!(p.episodes, vec![1, 2]);
    }

    #[test]
    fn parses_episode_range() {
        let p = parse_release("Show.S01E01-E03.1080p.WEB-DL-ABC");
        assert_eq!(p.episodes, vec![1, 2, 3]);
    }

    #[test]
    fn parses_season_pack() {
        let p = parse_release("Some Show Season 2 Complete 1080p BluRay x264-PACK");
        assert_eq!(p.season, Some(2));
        assert!(p.season_pack);
    }

    #[test]
    fn parses_movie_year() {
        let p = parse_release("Big.Movie.2024.2160p.BluRay.REMUX.HEVC-GROUP");
        assert_eq!(p.year, Some(2024));
        assert_eq!(p.resolution, Some(2160));
        assert_eq!(p.source.as_deref(), Some("REMUX"));
    }

    #[test]
    fn x_episode_format() {
        let p = parse_release("Old.Show.3x12.SDTV-XYZ");
        assert_eq!(p.season, Some(3));
        assert_eq!(p.episodes, vec![12]);
    }

    #[test]
    fn quality_prefers_bluray_over_cam() {
        let good = parse_release("Movie.2024.1080p.BluRay.x264-GRP");
        let bad = parse_release("Movie.2024.CAM.XviD-GRP");
        assert!(quality_rank(&good) > quality_rank(&bad));
    }

    #[test]
    fn episode_matching() {
        let p = parse_release("Show.S01E02.1080p.WEB-DL-GRP");
        assert!(matches_episode(&p, 1, &[2]));
        assert!(!matches_episode(&p, 1, &[3]));
        assert!(!matches_episode(&p, 2, &[2]));
        let pack = parse_release("Show.S01.COMPLETE.1080p.WEB-DL-GRP");
        assert!(matches_episode(&pack, 1, &[2, 3, 4]));
    }
}
