use crate::BundledAsset;

pub(crate) const ASSET_ROUTE_PREFIX: &str = "/_topcoat/assets";

pub type ResolveAssetRouteFn =
    dyn (Fn(&BundledAsset, &mut dyn std::fmt::Write) -> std::fmt::Result) + Send + Sync;

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
    ///
    /// # Errors
    ///
    /// Propagates errors of the registered [`ResolveAssetRouteFn`].
    pub fn resolve(
        &self,
        bundled_asset: &BundledAsset,
        write: &mut dyn std::fmt::Write,
    ) -> std::fmt::Result {
        (self.resolve_fn)(bundled_asset, write)
    }
}

impl Default for AssetRouteResolver {
    /// Builds the resolver used by
    /// [`RouterBuilderAssetExt::assets`](crate::RouterBuilderAssetExt::assets).
    fn default() -> Self {
        Self::new(Box::new(|bundled_asset, write| {
            write.write_str(ASSET_ROUTE_PREFIX)?;
            write.write_str("/")?;
            write.write_str(
                bundled_asset
                    .name()
                    .to_str()
                    .expect("asset needs UTF-8 name"),
            )
        }))
    }
}
