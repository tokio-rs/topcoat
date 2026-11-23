use bumpalo::Bump;
use topcoat_core::{context::Cx, error::Result};

use crate::{
    Endpoint, Handler, Layer, Path, PathBuf, PathSegment, RouteEntry, RouteId,
    error::redirect_permanent, request::uri, response::Response,
};

/// How the router treats a request for the other trailing-slash form of a
/// route's path.
///
/// A route declares whether its URL ends in `/`. This policy handles requests
/// for the other form. For example, it decides how `/users/` is handled when
/// the route is registered at `/users`.
///
/// Set it with [`RouterBuilder::trailing_slash`](crate::RouterBuilder::trailing_slash).
/// The default is [`Redirect`](Self::Redirect). It does not affect the root,
/// catch-all routes, or paths with both forms explicitly registered.
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
    /// The client repeats the request at the declared URL with the same
    /// method and body.
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
    pub(crate) fn register_twins<'r>(
        self,
        arena: &'r Bump,
        matcher: &mut matchit::Router<Endpoint<'r>>,
        endpoints: &[Endpoint<'r>],
        always_layers: &'r [&'r dyn Layer],
    ) {
        if self == Self::Strict {
            return;
        }
        for endpoint in endpoints {
            let Some(path) = twin(endpoint.path()) else {
                continue;
            };
            let matchit_path = path.to_matchit_path();
            let path: &'r Path = Path::new_unchecked(arena.alloc_str(path.as_str()));
            let twin = match self {
                Self::Serve => endpoint.with_path(path),
                _ => {
                    let mut twin = Endpoint::new(path);
                    twin.insert_any(arena.alloc(RouteEntry {
                        handler: Handler::Redirect {
                            id: RouteId::new(),
                            path,
                        },
                        layers: always_layers,
                        context: None,
                    }));
                    twin
                }
            };
            // A form a route claims for itself is left to that route.
            match matcher.insert(matchit_path, twin) {
                Ok(()) | Err(matchit::InsertError::Conflict { .. }) => {}
                Err(error) => panic!("failed to register route: {error}"),
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

/// Redirects a request that matched `path` to the same URL with its trailing
/// slash added or removed, keeping the query string.
pub(crate) fn redirect_to_twin(cx: &Cx, path: &Path) -> Result<Response> {
    // The request path matched `path`, so it has a trailing slash exactly
    // when `path` does.
    let request = uri(cx);
    let mut target = request.path().to_owned();
    if path.has_trailing_slash() {
        target.pop();
    } else {
        target.push('/');
    }
    if let Some(query) = request.query() {
        target.push('?');
        target.push_str(query);
    }
    Err(redirect_permanent(target).into())
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
