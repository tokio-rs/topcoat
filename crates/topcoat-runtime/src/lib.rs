//! Runtime types for Topcoat's client-side interactivity: [`Signal`]s,
//! runtime expressions ([`Expr`] and [`Js`]), event handler and bind
//! attributes, procedures, shards, and the values runtime expressions work
//! with.
//!
//! Use this crate through the `topcoat` facade, which re-exports it as
//! `topcoat::runtime` together with the runtime guide.
#![cfg_attr(docsrs, feature(doc_cfg))]

mod arguments;
mod bind_attribute;
mod event_handler;
mod expr;
mod js;
#[cfg(feature = "router")]
mod page;
#[cfg(feature = "router")]
mod procedure;
mod router;
#[cfg(feature = "router")]
mod shard;
#[cfg(feature = "router")]
mod shard_scope;
mod signal;
mod surrogate;

pub use arguments::*;
pub use bind_attribute::*;
pub use event_handler::*;
pub use expr::*;
pub use js::*;
#[cfg(feature = "router")]
pub use page::*;
#[cfg(feature = "router")]
pub use procedure::*;
pub use router::*;
#[cfg(feature = "router")]
pub use shard::*;
#[cfg(feature = "router")]
pub use shard_scope::*;
pub use signal::*;
pub use surrogate::*;
use topcoat_asset::{Asset, asset};

/// The browser runtime script, served as an asset.
///
/// Pages load it through a `<script type="module">` tag.
pub const SCRIPT: Asset = asset!("browser/dist/index.js", rename: "topcoat");
