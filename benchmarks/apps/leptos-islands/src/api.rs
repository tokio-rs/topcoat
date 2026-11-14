//! JSON API handlers, plain Axum routes next to the Leptos app. Server only.

mod cart;
mod error;
mod products;

pub use cart::*;
pub use error::*;
pub use products::*;
