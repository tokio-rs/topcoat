//! Runtime types for Topcoat's views: the [`View`] trait, rendered
//! [`ViewHandle`]s, and the traits that decide how values render in each
//! position of a `view!` template.
//!
//! Application code reaches these items through the `topcoat::view` module
//! of the [`topcoat`](https://docs.rs/topcoat) crate, together with the
//! `view!`, `class!`, and `attributes!` macros and the `#[component]`
//! attribute.

#![cfg_attr(docsrs, feature(doc_cfg))]

mod buffer;
mod child;
mod component;
mod css;
mod format;
mod hoist;
mod html;
mod props;
mod region;
mod string;
pub mod svg;
mod view;

pub use buffer::*;
pub use child::*;
pub use component::*;
pub use css::*;
pub use format::*;
pub use hoist::*;
pub use html::*;
pub use props::*;
pub use region::*;
pub use string::*;
pub use view::*;

/// Helpers called by macro-generated code. Not part of the public API.
#[doc(hidden)]
pub mod internal;
