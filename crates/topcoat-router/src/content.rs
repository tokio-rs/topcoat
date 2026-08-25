#![doc = include_str!("../docs/content.md")]
// Without the `multipart`, `sitemap`, `sse`, and `websocket` features the
// docs' links into those modules cannot resolve; they degrade to plain text
// instead of failing the build.
#![cfg_attr(
    not(all(
        feature = "multipart",
        feature = "sitemap",
        feature = "sse",
        feature = "websocket"
    )),
    allow(rustdoc::broken_intra_doc_links)
)]

mod css;
mod form;
mod html;
mod js;
mod json;
#[cfg(feature = "multipart")]
pub mod multipart;
#[cfg(feature = "sitemap")]
pub mod sitemap;
#[cfg(feature = "sse")]
pub mod sse;
mod view;
mod wasm;
#[cfg(feature = "websocket")]
pub mod websocket;

pub use css::*;
pub use form::*;
pub use html::*;
pub use js::*;
pub use json::*;
pub use wasm::*;
