use std::sync::Arc;

use topcoat_core::context::Cx;

use crate::{
    Body, Endpoint, Endpoints, Layer, Methods, Path, PathBuf, PathSegment, Route, RouteFuture,
    RouteId, Routes, error::redirect_permanent, request::uri,
};

/// How the router treats a request for the other trailing-slash form of a
/// route's path.
///
/// A route's path declares whether its URL ends in a slash: a page at
/// `/users` is served at `/users`, one at `/users/` at `/users/`. The policy
/// decides what a request for the form a route did not declare gets. It never
/// affects the root `/`, a route ending in a catch-all parameter, or a pair of
/// routes registered at both forms of one path. Set it with
/// [`RouterBuilder::trailing_slash`](crate::RouterBuilder::trailing_slash);
/// the default is [`Redirect`](Self::Redirect).
///
/// # Examples
///
/// ```rust
/// use topcoat::router::{Router, TrailingSlash};
///
/// let router = Router::builder()
///     .trailing_slash(TrailingSlash::Serve)
///     .build();
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TrailingSlash {
    /// Redirects to the declared form with a 308, keeping the query string.
    ///
    /// The status code preserves the method and the body, so a form posted
    /// to the other form of the URL is resubmitted to the declared one.
    #[default]
    Redirect,
    /// Serves the route under both forms. The client keeps the URL it asked
    /// for, and the handler reads it through [`uri`](crate::request::uri).
    Serve,
    /// Serves the route under its declared form only; the other form matches
    /// nothing and responds 404.
    Strict,
}

impl TrailingSlash {
    /// Registers the other trailing-slash form of every endpoint's path, so a
    /// request for it is handled under this policy: by the endpoint's own
    /// routes, or by a route redirecting to the declared form, which runs
    /// inside `always_layers` like a request that matched no route. A form a
    /// route claims for itself is left to that route.
    ///
    /// Call this once every route is registered.
    pub(crate) fn register_twins(
        self,
        endpoints: &mut Endpoints,
        routes: &mut Routes,
        always_layers: &[Arc<dyn Layer>],
    ) {
        if self == Self::Strict {
            return;
        }
        // The indices are taken up front, so the twins pushed along the way
        // are not visited themselves.
        for index in endpoints.indices() {
            let endpoint = &endpoints[index];
            let Some(path) = twin(endpoint.path()) else {
                continue;
            };
            let twin = match self {
                Self::Serve => endpoint.with_path(&path),
                _ => Endpoint::new(&path),
            };
            let Ok(twin_index) = endpoints.try_push(path.to_matchit_path(), twin) else {
                continue;
            };
            if self == Self::Redirect {
                let route = Box::new(RedirectRoute::new(path));
                let route = routes.push(route, twin_index, always_layers.into());
                endpoints[twin_index].insert_any(route);
            }
        }
    }
}

/// Returns the other trailing-slash form of `path`: `/users/` for `/users`,
/// and the other way around. The root and a path ending in a catch-all
/// parameter have no other form.
fn twin(path: &Path) -> Option<PathBuf> {
    let mut segments = path.segments();
    match segments.next_back()? {
        PathSegment::CatchAll(_) => None,
        PathSegment::Static("") => Some(segments.collect()),
        _ => {
            let mut twin = path.to_owned();
            twin += PathSegment::Static("");
            Some(twin)
        }
    }
}

/// The route serving the other trailing-slash form of an endpoint's path
/// under [`TrailingSlash::Redirect`], redirecting to the declared form.
struct RedirectRoute {
    id: RouteId,
    /// The other form, which this route is registered at.
    path: PathBuf,
}

impl RedirectRoute {
    fn new(path: PathBuf) -> Self {
        Self {
            id: RouteId::new(),
            path,
        }
    }
}

impl Route for RedirectRoute {
    fn id(&self) -> RouteId {
        self.id
    }

    fn methods(&self) -> Methods<'_> {
        Methods::Any
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, _body: Body) -> RouteFuture<'cx> {
        // The request path matched this route's path, so it has a trailing
        // slash exactly when the declared form does not.
        let request = uri(cx);
        let mut target = request.path().to_owned();
        if self.path.has_trailing_slash() {
            target.pop();
        } else {
            target.push('/');
        }
        if let Some(query) = request.query() {
            target.push('?');
            target.push_str(query);
        }
        Box::pin(async move { Err(redirect_permanent(target).into()) })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn twin_of(path: &'static str) -> Option<String> {
        twin(Path::new(path)).map(|twin| twin.to_string())
    }

    #[test]
    fn twin_adds_a_trailing_slash() {
        assert_eq!(twin_of("/users").as_deref(), Some("/users/"));
        assert_eq!(twin_of("/users/{id}").as_deref(), Some("/users/{id}/"));
    }

    #[test]
    fn twin_removes_a_trailing_slash() {
        assert_eq!(twin_of("/users/").as_deref(), Some("/users"));
        assert_eq!(twin_of("/users/{id}/").as_deref(), Some("/users/{id}"));
    }

    #[test]
    fn the_root_has_no_twin() {
        assert_eq!(twin_of("/"), None);
    }

    #[test]
    fn a_catch_all_has_no_twin() {
        assert_eq!(twin_of("/files/{*rest}"), None);
    }
}
