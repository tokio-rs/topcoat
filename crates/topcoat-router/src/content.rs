#![doc = include_str!("../docs/content.md")]
// Without the `multipart`, `sse`, and `websocket` features the docs' links into
// those modules cannot resolve; they degrade to plain text instead of failing
// the build.
#![cfg_attr(
    not(all(feature = "multipart", feature = "sse", feature = "websocket")),
    allow(rustdoc::broken_intra_doc_links)
)]

mod css;
mod form;
mod html;
mod json;
#[cfg(feature = "multipart")]
pub mod multipart;
#[cfg(feature = "sse")]
pub mod sse;
#[cfg(feature = "websocket")]
pub mod websocket;

pub use css::*;
pub use form::*;
pub use html::*;
pub use json::*;
