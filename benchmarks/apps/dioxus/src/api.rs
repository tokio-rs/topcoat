//! The public JSON API and the health check, as plain Axum handlers. Server only.

mod cart;
mod error;
mod products;

pub use cart::*;
pub use error::*;
pub use products::*;

/// `GET /health`: the readiness probe.
pub async fn health() -> &'static str {
    "ok"
}
