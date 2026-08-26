#![cfg_attr(docsrs, feature(doc_cfg))]

mod bind_attribute;
mod event_handler;
mod expr;
mod js;
#[cfg(feature = "router")]
mod procedure;
#[cfg(feature = "router")]
mod reactive_scope;
#[cfg(feature = "router")]
mod shard;
mod signal;
mod surrogate;

pub use bind_attribute::*;
pub use event_handler::*;
pub use expr::*;
pub use js::*;
#[cfg(feature = "router")]
pub use procedure::*;
#[cfg(feature = "router")]
pub use reactive_scope::*;
#[cfg(feature = "router")]
pub use shard::*;
pub use signal::*;
pub use surrogate::*;
use topcoat_asset::{Asset, asset};

pub const SCRIPT: Asset = asset!("browser/dist/index.js", rename: "topcoat");
