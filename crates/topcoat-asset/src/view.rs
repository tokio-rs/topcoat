use topcoat_core::context::{Cx, app_state};
use topcoat_view::runtime::{Formatter, Fragment};

use crate::Asset;

/// User-provided callback that turns an [`Asset`] into rendered output
/// (typically a URL or bundled path) when an [`Asset`] is used as a view
/// fragment.
pub type ResolveAssetFn = dyn Fn(&Cx, Asset, &mut Formatter<'_>) + Send + Sync;

/// App-state hook that lets [`Asset`] be rendered directly inside a view.
///
/// Install one into the app state before rendering any view that
/// contains an [`Asset`] fragment; without it, formatting will panic.
pub struct AssetFragmentResolver {
    resolve_fn: Box<ResolveAssetFn>,
}

impl AssetFragmentResolver {
    /// Build a resolver from a callback.
    pub fn new(resolve_fn: Box<ResolveAssetFn>) -> Self {
        Self { resolve_fn }
    }

    /// Invoke the underlying callback.
    pub fn resolve(&self, cx: &Cx, asset: Asset, f: &mut Formatter<'_>) {
        (self.resolve_fn)(cx, asset, f)
    }
}

impl Fragment for Asset {
    fn fmt(&self, cx: &Cx, f: &mut Formatter<'_>) {
        app_state::<AssetFragmentResolver>(cx).resolve(cx, *self, f)
    }
}
