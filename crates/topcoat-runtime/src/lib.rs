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
    use topcoat_view::{HtmlContext, PartsWriter, internal::__in_context};

    #[inline]
    pub fn __js(parts: &mut PartsWriter<'_>, js: impl Into<std::borrow::Cow<'static, str>>) {
        // JavaScript source renders inside comment markers and double-quoted
        // attributes; the comment context escapes the union of what both
        // positions need.
        __in_context(parts, HtmlContext::Comment, |parts| {
            match js.into() {
                std::borrow::Cow::Borrowed(js) => parts.push_static_str(js),
                std::borrow::Cow::Owned(js) => parts.push_string(js),
            };
        });
    }

    #[inline]
    pub fn __js_unescaped(parts: &mut PartsWriter<'_>, s: &'static str) {
        parts.push_static_str_unescaped(s);
    }

    #[inline]
    pub fn __surrogate(parts: &mut PartsWriter<'_>, value: &(impl serde::Serialize + ?Sized)) {
        __in_context(parts, HtmlContext::Comment, |parts| {
            parts.push_static_str_unescaped("cx.hydrate(");
            let json = serde_json::to_string(value).expect("failed to serialize surrogate value");
            parts.push_string(json);
            parts.push_static_str_unescaped(")");
        });
    }
}
