use topcoat_core::context::{Cx, app_state};
use topcoat_view::runtime::{AttributeValueViewParts, DynViewPart, FmtHtml, Formatter, ViewParts};

use crate::Asset;

/// User-provided callback that turns an [`Asset`] into rendered output
/// (typically a URL or bundled path) when an [`Asset`] is used as a view part.
pub type ResolveAssetFn = dyn Fn(&Cx, Asset, &mut Formatter<'_>) + Send + Sync;

/// App-state hook that lets [`Asset`] be rendered directly inside a view.
///
/// Install one into the app state before rendering any view that
/// contains an [`Asset`] fragment; without it, formatting will panic.
pub struct AssetResolver {
    resolve_fn: Box<ResolveAssetFn>,
}

impl AssetResolver {
    /// Build a resolver from a callback.
    pub fn new(resolve_fn: Box<ResolveAssetFn>) -> Self {
        Self { resolve_fn }
    }

    /// Invoke the underlying callback.
    pub fn resolve(&self, cx: &Cx, asset: Asset, f: &mut Formatter<'_>) {
        (self.resolve_fn)(cx, asset, f)
    }
}

impl FmtHtml for Asset {
    fn fmt_html(&self, cx: &Cx, f: &mut Formatter<'_>) {
        app_state::<AssetResolver>(cx).resolve(cx, *self, f)
    }
}

impl AttributeValueViewParts for Asset {
    #[inline]
    fn attribute_present(&self) -> bool {
        true
    }

    #[inline]
    fn into_view_parts(self, parts: &mut ViewParts) {
        parts.push(Box::new(self) as Box<dyn DynViewPart>);
    }
}
