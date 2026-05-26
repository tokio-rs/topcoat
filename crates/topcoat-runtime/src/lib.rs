pub mod ast;
mod bind_attribute;
mod event;
mod event_handler;
mod expr;
mod signal;
mod value;

pub use bind_attribute::*;
pub use event::*;
pub use event_handler::*;
pub use expr::*;
pub use signal::*;
pub use value::*;

use topcoat_asset::{Asset, asset};

pub const SCRIPT: Asset = asset!("browser/dist/index.mjs", rename: "topcoat");
