#![cfg_attr(docsrs, feature(doc_cfg))]

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
    use topcoat_view::{HtmlContext, PartsWriter, ViewParts};

    #[inline]
    pub fn __js(parts: &mut ViewParts, js: impl Into<std::borrow::Cow<'static, str>>) {
        // JavaScript source renders inside comment markers and double-quoted
        // attributes; the comment context escapes the union of what both
        // positions need.
        PartsWriter::new(parts, HtmlContext::Comment).push_str(js);
    }

    #[inline]
    pub fn __js_unescaped(parts: &mut ViewParts, s: &'static str) {
        PartsWriter::new(parts, HtmlContext::Unescaped).push_str(s);
    }

    #[inline]
    pub fn __surrogate(parts: &mut ViewParts, value: &(impl serde::Serialize + ?Sized)) {
        let mut writer = PartsWriter::new(parts, HtmlContext::Comment);
        writer.push_str_unescaped("cx.hydrate(");
        let json = serde_json::to_string(value).expect("failed to serialize surrogate value");
        writer.push_str(json);
        writer.push_str_unescaped(")");
    }
}
