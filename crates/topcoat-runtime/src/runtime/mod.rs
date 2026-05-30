mod bind_attribute;
mod event;
mod event_handler;
mod expr;
mod interop;
mod signal;

pub use bind_attribute::*;
pub use event::*;
pub use event_handler::*;
pub use expr::*;
pub use interop::*;
pub use signal::*;

use topcoat_asset::{Asset, asset};

pub const SCRIPT: Asset = asset!("browser/dist/index.mjs", rename: "topcoat");
