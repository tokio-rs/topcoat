use std::borrow::Cow;
use std::path::PathBuf as FsPathBuf;

use http::header::{CACHE_CONTROL, CONTENT_TYPE};
use http::{HeaderValue, Method, StatusCode};
use topcoat_core::runtime::context::Cx;
use topcoat_router::runtime::{Body, Path, PathBuf, Response, Route, RouteFuture, RouterBuilder};

use crate::{AssetBundle, AssetResolver, BundledAsset};

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

    fn path(&self) -> Cow<'static, Path> {
        Cow::Owned(self.path.clone())
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
    /// Mounts every file in `bundle` as a [`Route`] and installs the
    /// [`AssetResolver`] that turns [`Asset`](crate::Asset) values rendered in a
    /// [`View`](topcoat_view::runtime::View) into their bundled URLs.
    ///
    /// Without it, rendering a page that references an `Asset` panics.
    ///
    /// Load the bundle produced by `topcoat dev` or `topcoat asset bundle` with
    /// [`AssetBundle::load`] for the default location, or
    /// [`AssetBundle::load_dir`] for a custom one. The binary and the asset
    /// bundle must come from the same build: if a page renders an `Asset` that
    /// isn't present in the loaded bundle, rendering panics.
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

        let resolver = AssetResolver::new(Box::new(move |_cx, asset, f| match bundle.get(asset) {
            Some(asset) => {
                f.write_str(ASSET_ROUTE_PREFIX);
                f.write_str("/");
                f.write_str(asset.name().to_str().expect("asset had non-UTF8 name"));
            }
            None => panic!("failed to resolve asset {asset:?} in router's asset bundle"),
        }));

        self.app_context(resolver)
    }
}
