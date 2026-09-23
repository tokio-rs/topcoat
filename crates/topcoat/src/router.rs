#![doc = include_str!("../docs/router.md")]
// Links to optional APIs become plain text when their features are disabled.
#![cfg_attr(
    not(all(feature = "fs", feature = "serve", feature = "tower")),
    allow(rustdoc::broken_intra_doc_links)
)]

pub use topcoat_router::*;
pub use topcoat_router_macro::*;

/// Adds [`discover`](Self::discover) to [`RouterBuilder`].
///
/// See [auto-discovery](crate::router#auto-discovery-with-discover) in the
/// router guide.
#[cfg(feature = "discover")]
pub trait RouterBuilderDiscoverExt {
    /// Registers every item that was declared with a Topcoat attribute or
    /// macro anywhere in the program.
    ///
    /// This covers every `#[page]`, `#[layout]`, `#[layer]`, and `#[route]`,
    /// plus the items of enabled features that are collected the same way,
    /// like fonts, procedures, and shards.
    ///
    /// # Panics
    ///
    /// Panics if two discovered layouts, or two discovered layers, share the
    /// same path.
    #[must_use]
    fn discover(self) -> Self;
}

#[cfg(feature = "discover")]
impl RouterBuilderDiscoverExt for RouterBuilder {
    fn discover(mut self) -> Self {
        self = self.discover_routes();
        self = self.discover_pages();
        self = self.discover_layouts();
        self = self.discover_layers();
        #[cfg(feature = "font")]
        {
            use topcoat_font::RouterBuilderFontExt;
            self = self.discover_fonts();
        }
        #[cfg(feature = "runtime")]
        {
            use topcoat_runtime::{RouterBuilderProcedureExt, RouterBuilderShardExt};
            self = self.discover_procedures();
            self = self.discover_shards();
        }
        self
    }
}
