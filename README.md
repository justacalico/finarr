# Finarr

All-in-one media automation in a single binary: a BitTorrent client,
indexer search (Torznab · Jackett/Prowlarr), movie/series/music
libraries with monitoring and automatic import, plus Seerr-style
requests · all behind a polished Flutter web UI embedded right into the
binary. One executable, one database, zero config files: everything is
set up in the web UI.

## Run it

```bash
cargo build --release
./target/release/finarr
# open http://localhost:8787
```

Docker:

```bash
docker build -t finarr .
docker run -p 8787:8787 -v finarr-data:/config finarr
```

## What's inside

- **Torrents** · built-in BitTorrent engine (librqbit): magnet/file add,
  pause/resume/delete, per-file selection, global rate limits, DHT,
  UPnP, seed-ratio limits, persistent session.
- **Indexers** · any Torznab endpoint. Point a Jackett `/api/v2.0/indexers/.../results/torznab/` URL (or the `/all/` aggregator) plus your API key at it; test button verifies caps.
- **Movies** · TMDB lookup, monitored + wanted flow, automatic release
  search, grab, download, and clean import (`Title (Year)/Title (Year) [Quality].mkv`, hardlink by default).
- **Series** · TVmaze metadata with full episode lists, per-season/episode
  monitoring, season-pack preference, calendar for upcoming air dates.
- **Music** · MusicBrainz artists + release groups, monitored albums.
- **Requests** · users request media, admins approve (or auto-approve),
  the pipeline picks it up like any other wanted item.
- **Accounts** · first-run wizard, sessions, per-user API keys, admin roles. No env vars needed; all settings live in the Web UI.
- **Activity** · live queue merged with engine stats, full grab/import history, in-app server log viewer.
- **External clients** · optionally hand downloads to qBittorrent instead of the built-in engine.
- **Notifications** · ntfy, Discord, Gotify, or a generic webhook.

## Layout

- `src/` · Rust backend (axum + sqlx/SQLite + librqbit)
- `flutter/` · Flutter app for web, Android, iOS, Linux, Windows, macOS (same codebase everywhere)
- `frontend/dist/` · embedded web build output (built by `scripts/build-flutter.sh`)
- `scripts/` · release/CI helpers
- `migrations/` · SQLite schema

## Dev

```bash
cargo test            # backend tests
cd flutter && flutter test
cargo run -- --dev --local   # throwaway instance: random port, no login
```

## License

AGPL-3.0 · see LICENSE.
