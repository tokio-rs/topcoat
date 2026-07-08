mod bind_attribute;
mod event_handler;
mod expr;
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

/// Macro helpers to shorten the generated source code.
#[doc(hidden)]
pub mod internal {
    use topcoat_view::runtime::{NodeViewParts, Unescaped, ViewParts};

    #[inline]
    pub fn __js(parts: &mut ViewParts, js: &str) {
        parts.push(js.to_owned());
    }

    #[inline]
    pub fn __js_unescaped(parts: &mut ViewParts, s: &'static str) {
        Unescaped::new_unchecked(s).into_view_parts(parts);
    }

    #[inline]
    pub fn __surrogate(parts: &mut ViewParts, value: &(impl serde::Serialize + ?Sized)) {
        Unescaped::new_unchecked("cx.hydrate(").into_view_parts(parts);
        let json = serde_json::to_string(value).expect("failed to serialize surrogate value");
        parts.push(json);
        Unescaped::new_unchecked(")").into_view_parts(parts);
    }
}
