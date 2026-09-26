//! Embedded web UI: `frontend/dist` is baked into the binary at compile
//! time and served with an SPA fallback to index.html.

use axum::extract::Path;
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use include_dir::{include_dir, Dir};

static DIST: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/frontend/dist");

fn serve_file(path: &str) -> Option<Response> {
    let file = DIST.get_file(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    // Fingerprinted build artifacts can be cached forever; index.html must
    // always revalidate so deploys show up immediately.
    let cache = if path == "index.html" {
        "no-cache"
    } else {
        "public, max-age=31536000, immutable"
    };
    Some(
        (
            [
                (header::CONTENT_TYPE, HeaderValue::from_str(mime.as_ref()).ok()?),
                (header::CACHE_CONTROL, HeaderValue::from_static(cache)),
            ],
            file.contents(),
        )
            .into_response(),
    )
}

pub async fn static_handler(Path(path): Path<String>) -> Response {
    match serve_file(&path) {
        Some(r) => r,
        // SPA fallback: unknown non-file paths render the app shell.
        None => serve_file("index.html")
            .unwrap_or_else(|| StatusCode::NOT_FOUND.into_response()),
    }
}

pub async fn index_handler() -> Response {
    serve_file("index.html").unwrap_or_else(|| StatusCode::NOT_FOUND.into_response())
}
