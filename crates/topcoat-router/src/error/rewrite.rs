use std::{any::Any, sync::Mutex};

use http::{Method, uri::PathAndQuery};
use topcoat_core::context::RequestContext;

use crate::Body;

/// How many rewrites one request may consume before the router gives up and
/// responds 500.
pub(crate) const REWRITE_LIMIT: usize = 8;

/// Creates an internal rewrite that handles the request again at `path`.
///
/// When a handler returns the rewrite as an error, the router discards the
/// current response and routes the request again, as if the client had
/// requested `path`. `body` becomes the new request body. The method and
/// headers stay the same, and `path` may include a query string. Unlike a
/// redirect, the client does not see the rewrite: the browser keeps showing
/// the URL it requested. The handler at the new path can read that URL with
/// [`original_uri`](crate::request::original_uri).
///
/// A rewrite to a path and query the request was already routed to, or more
/// than 8 rewrites in a row, ends the request with a
/// `500 Internal Server Error`.
///
/// The returned [`RewriteError`] can also change the
/// [`method`](RewriteError::method) and pass values to the new
/// [request context](RewriteError::with).
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
        context: RequestContext::new(),
    }
}

/// An internal rewrite, returned as the error of a handler.
///
/// Create one with [`rewrite`]. The router catches it and routes the request
/// again at the new path instead of sending a response.
#[derive(Debug)]
pub struct RewriteError {
    path_and_query: PathAndQuery,
    /// The body for the rewritten dispatch. The mutex is never locked; it only
    /// makes the non-`Sync` [`Body`] shareable so the error can travel inside
    /// an [`Error`](topcoat_core::error::Error).
    body: Mutex<Body>,
    method: Option<Method>,
    context: RequestContext,
}

impl RewriteError {
    /// Sets the method of the rewritten request. By default the method stays
    /// the same.
    #[must_use]
    pub fn method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }

    /// Passes `value` to the rewritten request and to any later rewrites of
    /// it. Read it with
    /// [`request_context`](topcoat_core::context::request_context).
    ///
    /// Each rewrite starts with a new request context and an empty
    /// memoization cache, so only values passed with this method survive it.
    /// Passing a value of the same type again, on this rewrite or a later
    /// one, replaces the earlier value. Values that the router itself sets,
    /// such as the request parts and path parameters, win over passed values.
    #[must_use]
    pub fn with<T>(mut self, value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        self.context.insert(value);
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
            context: self.context,
        }
    }
}

/// The contents of a [`RewriteError`], taken apart for the router's rewrite
/// loop.
pub(crate) struct RewriteParts {
    pub(crate) path_and_query: PathAndQuery,
    pub(crate) body: Body,
    pub(crate) method: Option<Method>,
    pub(crate) context: RequestContext,
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
