use std::{any::Any, sync::Mutex};

use http::{HeaderMap, Method, request::Parts, uri::PathAndQuery};
use topcoat_core::{
    context::{ContextValues, RequestContext},
    error::Result,
};

use crate::{Body, request::Request};

/// How many rewrites one request may consume before the router gives up and
/// responds 500.
const REWRITE_LIMIT: usize = 8;

/// Builds an internal rewrite dispatching the request again at `path`.
///
/// Return this as an error to run the layers and handler at `path` with the
/// supplied body. The method and headers are preserved unless overridden,
/// and `path` may include a query string. The browser URL does not change.
/// Read it with [`original_uri`](crate::request::original_uri).
///
/// The router refuses a rewrite to a method and path the request was already
/// dispatched under, and stops a chain after 8 rewrites; either case responds
/// 500. Changing only the method, like a `POST` rewritten as a `GET` for the
/// same path, is a fresh dispatch.
///
/// Use [`RewriteError::method`] to change the method,
/// [`RewriteError::headers`] to replace the headers, and
/// [`RewriteError::with`] to carry values into the new request context.
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
        headers: None,
        context: RequestContext::new(),
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
    headers: Option<HeaderMap>,
    context: RequestContext,
}

impl RewriteError {
    /// Dispatches the rewritten request with `method` instead of the method
    /// the request arrived with.
    #[must_use]
    pub fn method(mut self, method: Method) -> Self {
        self.method = Some(method);
        self
    }

    /// Dispatches the rewritten request with `headers` instead of the
    /// headers the request arrived with.
    ///
    /// Start from a clone of [`headers`](crate::request::headers) to keep
    /// most of them, for example to drop the headers describing a body the
    /// rewrite replaces.
    #[must_use]
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers = Some(headers);
        self
    }

    /// Carries `value` into the rewritten dispatch and every dispatch after
    /// it in the same chain, available through
    /// [`request_context`](topcoat_core::context::request_context).
    ///
    /// Each dispatch starts with a fresh context and memoization cache.
    /// Only explicitly carried values survive a rewrite. Calling this
    /// again, or on a later rewrite, replaces a carried value of the same
    /// type. Values installed by the router, such as the request parts and
    /// path parameters, take precedence over carried values.
    #[must_use]
    pub fn with<T>(mut self, value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        self.context.insert(value);
        self
    }

    /// Splits the rewrite into what the next dispatch needs.
    fn into_parts(self) -> RewriteParts {
        let body = match self.body.into_inner() {
            Ok(body) => body,
            Err(poisoned) => poisoned.into_inner(),
        };
        RewriteParts {
            path_and_query: self.path_and_query,
            body,
            method: self.method,
            headers: self.headers,
            context: self.context,
        }
    }
}

/// The contents of a [`RewriteError`], taken apart for the rewrite chain.
struct RewriteParts {
    path_and_query: PathAndQuery,
    body: Body,
    method: Option<Method>,
    headers: Option<HeaderMap>,
    context: RequestContext,
}

impl std::fmt::Display for RewriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "rewrite to {}", self.path_and_query)
    }
}

impl std::error::Error for RewriteError {}

/// What a request remembers across the dispatches of a rewrite chain.
///
/// A request served in one dispatch leaves it empty. Each rewrite extends
/// it, and every dispatch after a rewrite starts its context from the
/// carried values.
#[derive(Default)]
pub(crate) struct RewriteChain {
    /// The dispatches the request went through so far, in order.
    visited: Vec<Dispatch>,
    /// The values rewrites carried into the later dispatches.
    context: RequestContext,
}

impl RewriteChain {
    /// The values the rewrites so far carry into the next dispatch.
    pub(crate) fn context(&self) -> &RequestContext {
        &self.context
    }

    /// Records the dispatch with `previous` as the one `rewrite` came out of
    /// and builds the request for the next dispatch, which keeps the headers
    /// and the method unless the rewrite changes them, under the rewritten
    /// path and query.
    ///
    /// Fails when the request already went through the rewritten dispatch
    /// or the chain grew past the rewrite limit. A rewrite that changes
    /// only the method is a fresh dispatch.
    pub(crate) fn follow(&mut self, previous: &Parts, rewrite: RewriteError) -> Result<Request> {
        let rewrite = rewrite.into_parts();
        self.visited.push(Dispatch::of(previous));
        let next = Dispatch {
            method: rewrite.method.unwrap_or_else(|| previous.method.clone()),
            path_and_query: rewrite.path_and_query,
        };
        if self.visited.contains(&next) {
            return Err(RewriteLoopError::cycle(&self.visited, &next).into());
        }
        if self.visited.len() > REWRITE_LIMIT {
            return Err(RewriteLoopError::limit(&self.visited, &next).into());
        }

        let mut parts = previous.clone();
        let mut uri = std::mem::take(&mut parts.uri).into_parts();
        uri.path_and_query = Some(next.path_and_query);
        parts.uri = http::Uri::from_parts(uri)
            .expect("replacing the path of a valid request uri keeps it valid");
        parts.method = next.method;
        if let Some(headers) = rewrite.headers {
            parts.headers = headers;
        }
        rewrite.context.install(&mut self.context);
        Ok(Request::from_parts(parts, rewrite.body))
    }
}

/// One dispatch of a request: its method and the path and query it ran
/// under.
#[derive(Debug, PartialEq, Eq)]
struct Dispatch {
    method: Method,
    path_and_query: PathAndQuery,
}

impl Dispatch {
    /// The dispatch `parts` describe.
    fn of(parts: &Parts) -> Self {
        let path_and_query = parts.uri.path_and_query().cloned().unwrap_or_else(|| {
            PathAndQuery::try_from(parts.uri.path())
                .expect("the path of a valid request uri is a valid path and query")
        });
        Self {
            method: parts.method.clone(),
            path_and_query,
        }
    }
}

impl std::fmt::Display for Dispatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.method, self.path_and_query)
    }
}

/// The failure that stops a runaway rewrite chain, responding 500.
///
/// The message records the chain of dispatches for error reporting; it is
/// never sent to the client.
#[derive(Debug)]
struct RewriteLoopError {
    message: String,
}

impl RewriteLoopError {
    /// A rewrite targeting a dispatch the request already went through.
    fn cycle(visited: &[Dispatch], target: &Dispatch) -> Self {
        Self {
            message: format!(
                "the rewrite to {target} creates a cycle: {} -> {target}",
                Self::chain(visited)
            ),
        }
    }

    /// A chain that ran past [`REWRITE_LIMIT`] without repeating a dispatch.
    fn limit(visited: &[Dispatch], target: &Dispatch) -> Self {
        Self {
            message: format!(
                "the request was rewritten more than {REWRITE_LIMIT} times: {} -> {target}",
                Self::chain(visited)
            ),
        }
    }

    /// Joins `visited` into one arrow-separated chain.
    fn chain(visited: &[Dispatch]) -> String {
        visited
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(" -> ")
    }
}

impl std::fmt::Display for RewriteLoopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for RewriteLoopError {}
