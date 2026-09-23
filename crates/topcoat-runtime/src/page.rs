use serde::Deserialize;
use topcoat_core::context::Cx;
use topcoat_router::{
    Body, Method, Methods, Path, PathBuf, Route, RouteFuture, RouteId,
    content::Json,
    error::rewrite,
    request::{FromRequest, uri},
};

use crate::SignalValues;

/// The route a page re-run is requested from, with the page's path and
/// query appended.
pub(crate) const PAGE_ROUTE_PREFIX: &str = "/_topcoat/runtime/pages";

/// The body of a request re-running a page: the current values of the
/// signals in the document.
#[derive(Debug, Deserialize)]
struct PageRerunRequest {
    #[serde(default)]
    signals: SignalValues,
}

/// A [`Route`] that re-runs a page with the signal values a client sends.
///
/// Re-renders the requested page through its layouts and guards, with the
/// browser's signal values on the request context.
pub struct PageRerunRoute {
    id: RouteId,
    path: PathBuf,
}

impl PageRerunRoute {
    /// The route re-running the root page.
    #[must_use]
    pub fn root() -> Self {
        Self {
            id: RouteId::new(),
            path: Path::new(PAGE_ROUTE_PREFIX).to_owned(),
        }
    }

    /// The route re-running every page below the root.
    #[must_use]
    pub fn nested() -> Self {
        Self {
            id: RouteId::new(),
            path: Path::new(&format!("{PAGE_ROUTE_PREFIX}/{{*page}}")).to_owned(),
        }
    }
}

impl Route for PageRerunRoute {
    fn id(&self) -> RouteId {
        self.id
    }

    fn methods(&self) -> Methods<'_> {
        // Avoids URL length limits for the signal values.
        Methods::Only(&[Method::POST])
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        Box::pin(async move {
            let Json(request) = Json::<PageRerunRequest>::from_request(cx, body).await?;

            // The page's path is what follows the prefix, and its query is
            // the request's own.
            let uri = uri(cx);
            let mut target = match uri.path().strip_prefix(PAGE_ROUTE_PREFIX) {
                Some(path) if !path.is_empty() => path.to_owned(),
                _ => String::from("/"),
            };
            if let Some(query) = uri.query() {
                target.push('?');
                target.push_str(query);
            }

            Err(rewrite(target, Body::empty())
                .method(Method::GET)
                .with(request.signals)
                .into())
        })
    }
}
