use std::sync::Mutex;

use http::uri::PathAndQuery;

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
}

impl RewriteError {
    /// Splits the rewrite into the path to dispatch and the body to dispatch
    /// it with.
    pub(crate) fn into_parts(self) -> (PathAndQuery, Body) {
        let body = match self.body.into_inner() {
            Ok(body) => body,
            Err(poisoned) => poisoned.into_inner(),
        };
        (self.path_and_query, body)
    }
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
