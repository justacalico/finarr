# Finarr

Single-binary media manager: Rust (axum + librqbit) backend with the
Flutter web UI embedded via `include_dir!`, plus Flutter clients for
Android/iOS/Linux/Windows/macOS, a separate landing page in `landing/`,
and a GitLab -> GitHub -> GitLab release pipeline (see `new-project`
skill notes and `.github/workflows/build.yml`).

## Layout

- `src/` Rust backend (`api/`, `media/`, `indexers/`, `clients/`, `meta/`,
  `services.rs`, `grab.rs`, `settings.rs`)
- `migrations/` SQLite migrations, embedded at build
- `frontend/dist/` generated Flutter web bundle (gitignored, embedded by
  `build.rs` which falls back to a placeholder page)
- `flutter/` the app (lib/, test/ with golden screenshots)
- `landing/` GitLab Pages site (own pubspec)
- `scripts/` CI helper scripts invoked by `.gitlab-ci.yml` and the
  GitHub workflow
- `altstore/source.json` AltStore source, refreshed by the pages job

## Verify

Run before committing and before considering work done:

    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    cargo test

    cd flutter && flutter analyze && flutter test
    cd landing && flutter analyze && flutter test

Frontend bundle: `bash scripts/build-flutter.sh` then rebuild the binary.
Dev server: `./finarr --dev --local` (auto user `dev/dev`... see config).
Rules: harsh review before merging, no god files, tests for new behavior,
merge commits not squashes. Commit messages are conventional commits with
Chinese subjects (`fix: ...`, `feat: ...`); `.githooks/commit-msg`
enforces the type prefix.
