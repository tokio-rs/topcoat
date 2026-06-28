use topcoat_core::runtime::context::{Cx, app_context};
use topcoat_view::runtime::{AttributeValueViewParts, DynViewPart, FmtHtml, Formatter, ViewParts};

use crate::{Asset, BundledAsset, bundled_asset};

pub type ResolveAssetRouteFn = dyn Fn(&BundledAsset, &mut dyn std::fmt::Write) + Send + Sync;

/// Function registered with the app context that formats the URL at which a bundled asset is
/// hosted by the router into a `dyn Write`.
pub struct AssetRouteResolver {
    resolve_fn: Box<ResolveAssetRouteFn>,
}

impl AssetRouteResolver {
    /// Build a resolver from a callback.
    #[must_use]
    pub fn new(resolve_fn: Box<ResolveAssetRouteFn>) -> Self {
        Self { resolve_fn }
    }

    /// Invoke the underlying callback.
    pub fn resolve(&self, bundled_asset: &BundledAsset, write: &mut dyn std::fmt::Write) {
        (self.resolve_fn)(bundled_asset, write);
    }
}

impl FmtHtml for Asset {
    fn fmt_html(&self, cx: &Cx, f: &mut Formatter<'_>) {
        let bundled_asset = bundled_asset(cx, *self);
        app_context::<AssetRouteResolver>(cx).resolve(bundled_asset, f);
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
