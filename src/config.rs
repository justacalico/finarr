//! Startup configuration: where data lives, and the dev-mode flags.
//! Everything else (ports, paths, indexers, keys) is configured in the
//! Web UI and stored in the database.

use std::net::IpAddr;
use std::path::PathBuf;

use anyhow::Result;

#[derive(Debug, Clone)]
pub struct Config {
    /// Root directory for the database, session persistence and defaults.
    pub data_dir: PathBuf,
    /// `--dev`: throwaway in-memory database, random port, no login.
    pub dev_mode: bool,
    /// `--local` (with `--dev`): bind loopback only instead of all interfaces.
    pub dev_local: bool,
    /// CLI port override; otherwise read from settings (default 8787).
    pub port_override: Option<u16>,
    /// CLI host override.
    pub host_override: Option<IpAddr>,
}

impl Config {
    pub fn db_url(&self) -> String {
        if self.dev_mode {
            "sqlite::memory:".to_string()
        } else {
            format!(
                "sqlite:{}?mode=rwc",
                self.data_dir.join("finarr.db").display()
            )
        }
    }

    pub fn torrent_state_dir(&self) -> PathBuf {
        self.data_dir.join("torrent-state")
    }
}

/// Parse CLI args. Supported: `--dev`, `--local`, `--data-dir <path>`,
/// `--port <n>`, `--host <addr>`.
pub fn parse_args(args: impl IntoIterator<Item = std::ffi::OsString>) -> Result<Config> {
    let mut data_dir = PathBuf::from("data");
    let mut dev_mode = false;
    let mut dev_local = false;
    let mut port_override = None;
    let mut host_override = None;

    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        let arg = arg.to_string_lossy();
        match arg.as_ref() {
            "--dev" => dev_mode = true,
            "--local" => dev_local = true,
            "--data-dir" => {
                if let Some(v) = it.next() {
                    data_dir = PathBuf::from(v);
                }
            }
            "--port" => {
                if let Some(v) = it.next() {
                    port_override = v.to_string_lossy().parse::<u16>().ok();
                }
            }
            "--host" => {
                if let Some(v) = it.next() {
                    host_override = v.to_string_lossy().parse::<IpAddr>().ok();
                }
            }
            _ => {
                if let Some((k, v)) = arg.split_once('=') {
                    match k {
                        "--data-dir" => data_dir = PathBuf::from(v),
                        "--port" => port_override = v.parse::<u16>().ok(),
                        "--host" => host_override = v.parse::<IpAddr>().ok(),
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(Config {
        data_dir,
        dev_mode,
        dev_local,
        port_override,
        host_override,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn args(items: &[&str]) -> Vec<OsString> {
        items.iter().map(OsString::from).collect()
    }

    #[test]
    fn defaults() {
        let cfg = parse_args(args(&[])).unwrap();
        assert!(!cfg.dev_mode);
        assert_eq!(cfg.data_dir, PathBuf::from("data"));
        assert_eq!(cfg.db_url(), "sqlite:data/finarr.db?mode=rwc");
    }

    #[test]
    fn dev_mode_flags() {
        let cfg = parse_args(args(&["--dev", "--local"])).unwrap();
        assert!(cfg.dev_mode && cfg.dev_local);
        assert_eq!(cfg.db_url(), "sqlite::memory:");
    }

    #[test]
    fn flag_values() {
        let cfg = parse_args(args(&[
            "--data-dir",
            "/srv/finarr",
            "--port=9090",
            "--host",
            "0.0.0.0",
        ]))
        .unwrap();
        assert_eq!(cfg.data_dir, PathBuf::from("/srv/finarr"));
        assert_eq!(cfg.port_override, Some(9090));
        assert_eq!(cfg.host_override, Some("0.0.0.0".parse().unwrap()));
    }
}
