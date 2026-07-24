use topcoat_router::RouterBuilder;

#[cfg(feature = "serve")]
use crate::AssetRoute;
use crate::config::Host;
#[cfg(feature = "serve")]
use crate::serve::ASSET_ROUTE_PREFIX;
use crate::{AssetRouteResolver, Config};

/// Registers assets on a [`RouterBuilder`].
///
/// Implemented for [`RouterBuilder`] so it is in scope wherever a router is
/// being built, enabling the [`assets`](Self::assets) method.
pub trait RouterBuilderAssetExt {
    /// Registers an asset [`Config`] on the router.
    ///
    /// The configuration's bundle is registered with the app context, allowing
    /// access through [`asset_bundle`](crate::asset_bundle) and
    /// [`bundled_asset`](crate::bundled_asset), and [`Asset`](crate::Asset)
    /// handles used as attribute values in the `view!` macro get rendered as
    /// the URL the asset is hosted at.
    ///
    /// With the default hosting, each asset in the bundle is also added as an
    /// HTTP route and served by the application itself. A configuration hosted
    /// externally (see
    /// [`ConfigBuilder::hosted_at`](crate::ConfigBuilder::hosted_at)) adds no
    /// routes; the bundled files must be hosted at the configured base URL by
    /// other means.
    ///
    /// Anything convertible into a [`Config`] is accepted: an [`AssetBundle`]
    /// registers that bundle with the default hosting.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[topcoat::router::page("/")] async fn about() -> topcoat::Result { topcoat::view::view! {} }
    /// use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
    /// use topcoat::router::Router;
    ///
    /// pub fn router() -> Router {
    ///     Router::builder()
    ///         .page(about)
    ///         .assets(AssetBundle::load().unwrap())
    ///         .build()
    /// }
    /// ```
    ///
    /// Hosting the bundled files on a CDN instead of serving them:
    ///
    /// ```rust
    /// use topcoat::asset::{Config, RouterBuilderAssetExt};
    /// use topcoat::router::Router;
    ///
    /// pub fn router() -> Router {
    ///     Router::builder()
    ///         .assets(Config::builder().hosted_at("https://cdn.example.com/assets").build())
    ///         .build()
    /// }
    /// ```
    ///
    /// [`AssetBundle`]: crate::AssetBundle
    #[must_use]
    fn assets(self, config: impl Into<Config>) -> Self;
}

impl RouterBuilderAssetExt for RouterBuilder {
    fn assets(mut self, config: impl Into<Config>) -> Self {
        let Config { bundle, host } = config.into();

        let base_url = match host {
            #[cfg(feature = "serve")]
            Host::Serve => {
                for asset in bundle.assets() {
                    self = self.route(AssetRoute::new(asset));
                }
                ASSET_ROUTE_PREFIX.to_owned()
            }
            Host::External { base_url } => base_url,
        };

        self = self.app_context(bundle);
        self = self.app_context(AssetRouteResolver::new(Box::new(
            move |bundled_asset, write| {
                write.write_str(&base_url)?;
                write.write_str("/")?;
                write.write_str(
                    bundled_asset
                        .name()
                        .to_str()
                        .expect("asset needs UTF-8 name"),
                )?;
                Ok(())
            },
        )));

        self
    }
}
