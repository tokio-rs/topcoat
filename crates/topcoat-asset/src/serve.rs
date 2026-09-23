use std::path::PathBuf as FsPathBuf;

use http::{
    HeaderValue, Method, StatusCode,
    header::{CACHE_CONTROL, CONTENT_TYPE},
};
use topcoat_core::context::Cx;
use topcoat_router::{
    Body, Methods, Path, PathBuf, Route, RouteFuture, RouteId, response::Response,
};

use crate::BundledAsset;

/// The URL path that every asset served by the application starts with.
pub(crate) const ASSET_ROUTE_PREFIX: &str = "/_topcoat/assets";

/// The `Cache-Control` value of every served asset. Bundled filenames contain
/// a content hash, so the contents behind a URL never change.
const CACHE_CONTROL_VALUE: HeaderValue =
    HeaderValue::from_static("public, max-age=31536000, immutable");

/// A [`Route`] that serves one bundled file from disk.
///
/// The route answers `GET` requests at `/_topcoat/assets/{filename}`. It
/// reads the file on each request and responds with the file's
/// `Content-Type` and a `Cache-Control` header that marks the response as
/// immutable for a year. If the file cannot be read, it responds with
/// `404 Not Found`.
///
/// You usually do not create this route yourself. Registering a bundle with
/// the router's `assets` method adds one route per bundled file.
#[derive(Debug, Clone)]
pub struct AssetRoute {
    /// The identity of this route's handler.
    id: RouteId,
    /// URL path the asset is served at, e.g. `/_topcoat/assets/logo-1a2b3c4d5e6f7a8b.png`.
    path: PathBuf,
    /// Absolute path to the bundled file on disk.
    file: FsPathBuf,
    /// Content type specified in the manifest.
    content_type: HeaderValue,
}

impl AssetRoute {
    /// Creates the route that serves `asset` from the bundle directory `dir`.
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
            id: RouteId::new(),
            path: Path::new(&format!("{ASSET_ROUTE_PREFIX}/{name}")).to_owned(),
            file: dir.join(name),
            content_type,
        }
    }
}

impl Route for AssetRoute {
    fn id(&self) -> RouteId {
        self.id
    }

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
