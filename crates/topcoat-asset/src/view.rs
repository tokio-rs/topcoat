use topcoat_core::context::{Cx, app_state};
use topcoat_view::runtime::{Formatter, Fragment};

use crate::Asset;

pub type ResolveAssetFn = dyn Fn(&Cx, Asset, &mut Formatter<'_>) + Send + Sync;

pub struct AssetFragmentResolver {
    resolve_fn: Box<ResolveAssetFn>,
}

impl AssetFragmentResolver {
    pub fn new(resolve_fn: Box<ResolveAssetFn>) -> Self {
        Self { resolve_fn }
    }

    pub fn resolve(&self, cx: &Cx, asset: Asset, f: &mut Formatter<'_>) {
        (self.resolve_fn)(cx, asset, f)
    }
}

impl Fragment for Asset {
    fn fmt(&self, cx: &Cx, f: &mut Formatter<'_>) {
        app_state::<AssetFragmentResolver>(cx).resolve(cx, *self, f)
    }
}
