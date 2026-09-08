use std::sync::Mutex;

use http::{Method, uri::PathAndQuery};
use topcoat_core::context::Cx;

use crate::Body;

/// How many rewrites one request may consume before the router gives up and
/// responds 500.
pub(crate) const REWRITE_LIMIT: usize = 8;

/// Builds an internal rewrite dispatching the request again at `path`.
///
/// Returning it from a handler makes the router run the whole route stack
/// again as if `path` had been requested in the first place, with `body` as
/// the request body. The method and headers carry over unchanged, and `path`
/// may include a query string. Unlike a redirect, the substitution is
/// invisible to the client: the browser URL stays the URL that was requested.
/// The handler at the rewritten path can read that original URL with
/// [`original_uri`](crate::request::original_uri).
///
/// The router refuses a rewrite to a path the request was already dispatched
/// under, and stops a chain after 8 rewrites; either case responds 500.
///
/// The returned [`RewriteError`] has options for the rare cases where the
/// rewritten dispatch should differ from the request in more than its path
/// and body: another [`method`](RewriteError::method), or values carried on
/// its [request context](RewriteError::cx).
///
/// # Panics
///
/// Panics if `path` is not a valid URI path and query.
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{Body, error::rewrite, page},
///     view::{View, view},
/// };
/// # async fn beta_tester(_cx: &Cx) -> bool { false }
///
/// #[page("/dashboard")]
/// async fn dashboard(cx: &Cx) -> Result<impl View> {
///     if beta_tester(cx).await {
///         return Err(rewrite("/dashboard-beta", Body::empty()).into());
///     }
///     Ok(view! { <h1>"Dashboard"</h1> })
/// }
/// ```
#[must_use]
#[track_caller]
pub fn rewrite(path: impl AsRef<str>, body: impl Into<Body>) -> RewriteError {
    RewriteError {
        path_and_query: PathAndQuery::try_from(path.as_ref())
            .expect("rewrite path is not a valid uri path and query"),
        body: Mutex::new(body.into()),
        method: None,
        cx: None,
    }
}

/// An internal rewrite carried as the `Err` variant of a handler `Result`.
///
/// Construct one with [`rewrite`]. The router intercepts it and dispatches
/// the request again at the carried path instead of sending a response.
#[derive(Debug)]
pub struct RewriteError {
    path_and_query: PathAndQuery,
    /// The body for the rewritten dispatch. The mutex is never locked; it only
    /// makes the non-`Sync` [`Body`] shareable so the error can travel inside
    /// an [`Error`](topcoat_core::error::Error).
    body: Mutex<Body>,
    method: Option<Method>,
    cx: Option<Cx>,
}

impl RewriteError {
    /// Dispatches the rewritten request with `method` instead of the method
    /// the request arrived with.
    #[must_use]
    pub fn method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }

    /// Carries the request context values of `cx` into the rewritten
    /// dispatch, and every dispatch after it in the same chain.
    ///
    /// A dispatch normally starts from an empty request context. With this
    /// option it starts from the values on `cx`, so a handler can hand
    /// values it computed to the handler at the target, typically through
    /// [`Cx::with`]. The values the router installs for a dispatch, such as
    /// the request parts and the path parameters, shadow any of the same
    /// type carried over.
    #[must_use]
    pub fn cx(mut self, cx: Cx) -> Self {
        self.cx = Some(cx);
        self
    }

    /// Splits the rewrite into what the next dispatch needs.
    pub(crate) fn into_parts(self) -> RewriteParts {
        let body = match self.body.into_inner() {
            Ok(body) => body,
            Err(poisoned) => poisoned.into_inner(),
        };
        RewriteParts {
            path_and_query: self.path_and_query,
            body,
            method: self.method,
            cx: self.cx,
        }
    }
}

/// The contents of a [`RewriteError`], taken apart for the router's rewrite
/// loop.
pub(crate) struct RewriteParts {
    pub(crate) path_and_query: PathAndQuery,
    pub(crate) body: Body,
    pub(crate) method: Option<Method>,
    pub(crate) cx: Option<Cx>,
}

impl std::fmt::Display for RewriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rewrite to {}", self.path_and_query)
    }
}

impl std::error::Error for RewriteError {}

/// The failure that stops a runaway rewrite chain, responding 500.
///
/// The message records the chain of paths for error reporting; it is never
/// sent to the client.
#[derive(Debug)]
pub(crate) struct RewriteLoopError {
    message: String,
}

impl RewriteLoopError {
    /// A rewrite targeting a path the request was already dispatched under.
    pub(crate) fn cycle(visited: &[String], target: &str) -> Self {
        Self {
            message: format!(
                "the rewrite to {target} creates a cycle: {} -> {target}",
                visited.join(" -> ")
            ),
        }
    }

    /// A chain that ran past [`REWRITE_LIMIT`] without repeating a path.
    pub(crate) fn limit(visited: &[String], target: &str) -> Self {
        Self {
            message: format!(
                "the request was rewritten more than {REWRITE_LIMIT} times: {} -> {target}",
                visited.join(" -> ")
            ),
        }
    }
}

impl std::fmt::Display for RewriteLoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RewriteLoopError {}
