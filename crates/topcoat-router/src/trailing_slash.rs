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
