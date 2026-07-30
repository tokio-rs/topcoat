use std::path::PathBuf as FsPathBuf;

use http::header::{CACHE_CONTROL, CONTENT_TYPE};
use http::{HeaderValue, Method, StatusCode};
use topcoat_core::context::Cx;
use topcoat_router::{Body, Methods, Path, PathBuf, Response, Route, RouteFuture};

use crate::BundledAsset;

/// URL prefix every application-served asset is hosted under.
pub(crate) const ASSET_ROUTE_PREFIX: &str = "/_topcoat/assets";

/// `Cache-Control` applied to every served asset. Bundled filenames carry a
/// content hash, so their contents never change for a given URL.
const CACHE_CONTROL_VALUE: HeaderValue =
    HeaderValue::from_static("public, max-age=31536000, immutable");

/// A [`Route`] that serves a single bundled asset from disk.
///
/// One is registered per [`BundledAsset`] by the router's `assets` extension
/// method when the configuration serves the bundle from the application; the
/// route reads the file on demand and responds with the appropriate
/// `Content-Type` and an immutable `Cache-Control`.
#[derive(Debug, Clone)]
pub struct AssetRoute {
    /// URL path the asset is served at, e.g. `/_topcoat/assets/logo-1a2b3c4d5e6f7a8b.png`.
    path: PathBuf,
    /// Absolute path to the bundled file on disk.
    file: FsPathBuf,
    /// Content type specified in the manifest.
    content_type: HeaderValue,
}

impl AssetRoute {
    /// Builds the route that serves `asset` out of the bundle directory `dir`.
    ///
    /// # Panics
    ///
    /// Panics if the asset's `Content-Type` cannot be converted into a
    /// [`HeaderValue`].
    #[must_use]
    #[track_caller]
    pub fn new(dir: &std::path::Path, asset: &BundledAsset) -> Self {
        let name = asset.name();
        let content_type = HeaderValue::from_str(asset.content_type()).unwrap_or_else(|_| {
            panic!(
                "asset `{}` has Content-Type \"{}\" that cannot be converted into a header value",
                name,
                asset.content_type()
            )
        });
        Self {
            path: Path::new(&format!("{ASSET_ROUTE_PREFIX}/{name}")).to_owned(),
            file: dir.join(name),
            content_type,
        }
    }
}

impl Route for AssetRoute {
    fn methods(&self) -> Methods<'_> {
        Methods::Only(&[Method::GET])
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
