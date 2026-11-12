//! The stylesheet and script, embedded in the binary and served under content-hashed URLs.

use axum::{
    Router,
    http::header::{CACHE_CONTROL, CONTENT_TYPE},
    response::IntoResponse,
    routing::get,
};

include!(concat!(env!("OUT_DIR"), "/asset_paths.rs"));

const STYLESHEET: &str = include_str!(concat!(env!("OUT_DIR"), "/app.css"));
const SCRIPT: &str = include_str!("../assets/app.js");
const IMMUTABLE: &str = "public, max-age=31536000, immutable";

/// Routes for the embedded assets.
pub fn router<S: Clone + Send + Sync + 'static>() -> Router<S> {
    Router::new()
        .route(STYLESHEET_PATH, get(stylesheet))
        .route(SCRIPT_PATH, get(script))
}

async fn stylesheet() -> impl IntoResponse {
    (
        [
            (CONTENT_TYPE, "text/css; charset=utf-8"),
            (CACHE_CONTROL, IMMUTABLE),
        ],
        STYLESHEET,
    )
}

async fn script() -> impl IntoResponse {
    (
        [
            (CONTENT_TYPE, "text/javascript; charset=utf-8"),
            (CACHE_CONTROL, IMMUTABLE),
        ],
        SCRIPT,
    )
}
