//! Filesystem naming: library folder layout for each media type.
//!
//!   Movies:  <root>/<Title> (<Year>)/<Title> (<Year>) [<Quality>].<ext>
//!   Series:  <root>/<Title>/Season <SS>/<Title> - S<SS>E<EE> - <EpTitle> [<Quality>].<ext>
//!   Music:   <root>/<Artist>/<Album> (<Year>)/<files>

use std::path::{Path, PathBuf};

/// Strip characters that are illegal or annoying on common filesystems.
pub fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => ' ',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let squashed = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");
    squashed.trim_matches(|c| c == '.' || c == ' ').to_string()
}

pub fn movie_folder(root: &Path, title: &str, year: Option<i64>) -> PathBuf {
    let name = match year {
        Some(y) => format!("{} ({})", sanitize(title), y),
        None => sanitize(title),
    };
    root.join(name)
}

pub fn movie_file(title: &str, year: Option<i64>, quality: &str, ext: &str) -> String {
    let base = match year {
        Some(y) => format!("{} ({})", sanitize(title), y),
        None => sanitize(title),
    };
    if quality.is_empty() {
        format!("{base}.{ext}")
    } else {
        format!("{base} [{}].{ext}", sanitize(quality))
    }
}

pub fn series_folder(root: &Path, title: &str) -> PathBuf {
    root.join(sanitize(title))
}

pub fn season_folder(title: &str, season: u32) -> String {
    let _ = title;
    if season == 0 {
        "Specials".to_string()
    } else {
        format!("Season {season:02}")
    }
}

pub fn episode_file(
    title: &str,
    season: u32,
    episodes: &[u32],
    ep_title: &str,
    quality: &str,
    ext: &str,
) -> String {
    let mut name = sanitize(title);
    let ep_code = episodes
        .iter()
        .map(|e| format!("E{e:02}"))
        .collect::<Vec<_>>()
        .join("");
    name.push_str(&format!(" - S{season:02}{ep_code}"));
    if !ep_title.is_empty() {
        name.push_str(&format!(" - {}", sanitize(ep_title)));
    }
    if !quality.is_empty() {
        name.push_str(&format!(" [{}]", sanitize(quality)));
    }
    format!("{name}.{ext}")
}

pub fn album_folder(root: &Path, artist: &str, album: &str, year: Option<i64>) -> PathBuf {
    let album_name = match year {
        Some(y) if y > 0 => format!("{} ({})", sanitize(album), y),
        _ => sanitize(album),
    };
    root.join(sanitize(artist)).join(album_name)
}

const VIDEO_EXTS: &[&str] = &[
    "mkv", "mp4", "m4v", "avi", "wmv", "mov", "ts", "m2ts", "webm", "mpg", "mpeg",
];
const AUDIO_EXTS: &[&str] = &[
    "flac", "mp3", "aac", "m4a", "ogg", "opus", "wav", "wma", "alac",
];
const SUB_EXTS: &[&str] = &["srt", "ass", "ssa", "sub", "vtt"];

pub fn ext_of(path: &Path) -> String {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

pub fn is_video(path: &Path) -> bool {
    VIDEO_EXTS.contains(&ext_of(path).as_str())
}

pub fn is_audio(path: &Path) -> bool {
    AUDIO_EXTS.contains(&ext_of(path).as_str())
}

pub fn is_subtitle(path: &Path) -> bool {
    SUB_EXTS.contains(&ext_of(path).as_str())
}

/// True for junk filenames scene releases love to ship.
pub fn is_sample_or_junk(path: &Path) -> bool {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    name.contains("sample")
        || name.ends_with(".nfo")
        || name.ends_with(".sfv")
        || name.ends_with(".nzb")
        || name.ends_with(".txt") && name.contains("read")
        || name.contains("rarbg.com")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_names() {
        assert_eq!(sanitize("Foo: Bar/Baz?"), "Foo Bar Baz");
        assert_eq!(sanitize("  spaced   out  "), "spaced out");
    }

    #[test]
    fn movie_paths() {
        let f = movie_folder(Path::new("/m"), "The Movie", Some(2024));
        assert_eq!(f, Path::new("/m/The Movie (2024)"));
        let n = movie_file("The Movie", Some(2024), "WEB-DL 1080p", "mkv");
        assert_eq!(n, "The Movie (2024) [WEB-DL 1080p].mkv");
    }

    #[test]
    fn episode_names() {
        let n = episode_file("Show", 2, &[5], "The Episode", "1080p", "mkv");
        assert_eq!(n, "Show - S02E05 - The Episode [1080p].mkv");
        let m = episode_file("Show", 1, &[1, 2], "", "", "mkv");
        assert_eq!(m, "Show - S01E01E02.mkv");
    }

    #[test]
    fn file_kinds() {
        assert!(is_video(Path::new("a.MKV")));
        assert!(is_audio(Path::new("a.flac")));
        assert!(is_subtitle(Path::new("a.srt")));
        assert!(!is_video(Path::new("a.nfo")));
        assert!(is_sample_or_junk(Path::new("x.sample.mkv")));
    }
}
