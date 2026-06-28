use std::path::PathBuf as FsPathBuf;

use http::header::{CACHE_CONTROL, CONTENT_TYPE};
use http::{HeaderValue, Method, StatusCode};
use topcoat_core::runtime::context::Cx;
use topcoat_router::runtime::{Body, Path, PathBuf, Response, Route, RouteFuture, RouterBuilder};

use crate::{AssetBundle, AssetRouteResolver, BundledAsset};

/// URL prefix every bundled asset is served under.
const ASSET_ROUTE_PREFIX: &str = "/_topcoat/assets";

/// `Cache-Control` applied to every served asset. Bundled filenames carry a
/// content hash, so their contents never change for a given URL.
const CACHE_CONTROL_VALUE: HeaderValue =
    HeaderValue::from_static("public, max-age=31536000, immutable");

/// A [`Route`] that serves a single bundled asset from disk.
///
/// One is registered per [`BundledAsset`] by
/// [`RouterBuilderAssetExt::assets`]; the route reads the file on demand and
/// responds with the appropriate `Content-Type` and an immutable
/// `Cache-Control`.
#[derive(Debug, Clone)]
pub struct AssetRoute {
    /// URL path the asset is served at, e.g. `/_topcoat/assets/logo-1a2b3c4d.png`.
    path: PathBuf,
    /// Absolute path to the bundled file on disk.
    file: FsPathBuf,
    /// Content type specified in the manifest.
    content_type: HeaderValue,
}

impl AssetRoute {
    /// Builds the route that serves `asset`.
    ///
    /// # Panics
    ///
    /// Panics if the asset's filename is not valid UTF-8, or if its
    /// `Content-Type` cannot be converted into a [`HeaderValue`].
    #[must_use]
    pub fn new(asset: &BundledAsset) -> Self {
        let name = asset.name().to_str().expect("asset had non-UTF8 name");
        let content_type = HeaderValue::from_str(asset.content_type()).unwrap_or_else(|_| {
            panic!(
                "asset `{}` has Content-Type \"{}\" that cannot be converted into a header value",
                name,
                asset.content_type()
            )
        });
        Self {
            path: Path::new(&format!("{ASSET_ROUTE_PREFIX}/{name}")).to_owned(),
            file: asset.path().to_path_buf(),
            content_type,
        }
    }
}

impl Route for AssetRoute {
    fn method(&self) -> Method {
        Method::GET
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'cx>(&'cx self, _cx: &'cx Cx, _body: Body) -> RouteFuture<'cx> {
        Box::pin(async move {
            let response = if let Ok(bytes) = tokio::fs::read(&self.file).await {
                let mut response = Response::new(Body::from(bytes));
                let headers = response.headers_mut();
                headers.insert(CONTENT_TYPE, self.content_type.clone());
                headers.insert(CACHE_CONTROL, CACHE_CONTROL_VALUE);
                response
            } else {
                let mut response = Response::new(Body::empty());
                *response.status_mut() = StatusCode::NOT_FOUND;
                response
            };
            Ok(response)
        })
    }
}

/// Registers an [`AssetBundle`] on a [`RouterBuilder`].
///
/// Implemented for [`RouterBuilder`] so it is in scope wherever a router is
/// being built, enabling the [`assets`](Self::assets) method.
pub trait RouterBuilderAssetExt {
    /// Sets the router's asset bundle.
    ///
    /// This hosts each asset in `bundle` as an HTTP route in the router.
    /// It also registers the asset bundle with the app context, allowing access through
    /// [`asset_bundle`] and [`bundled_asset`].
    /// Additionally, [`Asset`] handles used as attribute values in the `view!` macro
    /// get rendered as the URL path the asset is hosted at.
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
    #[must_use]
    fn assets(self, bundle: AssetBundle) -> Self;
}

impl RouterBuilderAssetExt for RouterBuilder {
    fn assets(mut self, bundle: AssetBundle) -> Self {
        for asset in bundle.assets() {
            self = self.route(AssetRoute::new(asset));
        }

        self = self.app_context(bundle);
        self = self.app_context(AssetRouteResolver::new(Box::new(|bundled_asset, write| {
            write.write_str(ASSET_ROUTE_PREFIX)?;
            write.write_str("/")?;
            write.write_str(
                bundled_asset
                    .name()
                    .to_str()
                    .expect("asset needs UTF-8 name"),
            )?;
            Ok(())
        })));

        self
    }
}
