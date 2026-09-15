#![doc = include_str!("../docs/strip_prefix.md")]
#![cfg_attr(not(feature = "serve"), allow(rustdoc::broken_intra_doc_links))]

use std::borrow::Cow;

use topcoat_core::context::Cx;

use crate::{
    Body, IntoPath, Layer, LayerFuture, Next, Path,
    request::{parts, uri},
};

/// A [`Layer`] that removes a leading URI path prefix before inner layers and
/// the route run.
///
/// Create it with [`new`](Self::new), whose argument is both the prefix
/// stripped from each request and the path of matched routes the layer wraps.
/// Scope it to a different set of routes with [`at`](Self::at).
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     router::{Router, StripPrefix, route},
/// };
///
/// #[route(GET "/res/{*path}")]
/// async fn files() -> Result<&'static str> {
///     Ok("ok")
/// }
///
/// let router = Router::builder()
///     .route(files)
///     .layer(StripPrefix::new("/res"))
///     .build();
/// ```
#[derive(Debug, Clone)]
pub struct StripPrefix {
    /// The URL path prefix whose matched routes this layer wraps.
    path: Cow<'static, Path>,
    /// The URI path prefix stripped from each request.
    prefix: Cow<'static, str>,
}

impl StripPrefix {
    /// Strips `prefix` from the request path for the matched routes under it.
    ///
    /// `prefix` is a route path (`/res`), used both as the layer's path and as
    /// the URI prefix to remove. A request at `/res/hello.txt` is rewritten to
    /// `/hello.txt` for inner layers and the route.
    ///
    /// # Panics
    ///
    /// Panics if `prefix` is a string that is not a well-formed route path.
    #[must_use]
    #[track_caller]
    pub fn new(prefix: impl IntoPath) -> Self {
        let path = prefix.into_path();
        let prefix = if path.is_empty() {
            Cow::Borrowed("")
        } else {
            Cow::Owned(path.as_str().to_owned())
        };
        Self { path, prefix }
    }

    /// Scopes the layer to the matched routes under `path` without changing
    /// the URI prefix it strips.
    ///
    /// # Panics
    ///
    /// Panics if `path` is a string that is not a well-formed route path.
    #[must_use]
    #[track_caller]
    pub fn at(mut self, path: impl IntoPath) -> Self {
        self.path = path.into_path();
        self
    }
}

impl Layer for StripPrefix {
    fn path(&self) -> Option<&Path> {
        Some(&self.path)
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            let Some(stripped) = strip_path_prefix(uri(cx).path(), &self.prefix) else {
                return next.run(cx, body).await;
            };
            let mut parts = parts(cx).clone();
            parts.uri = rewrite_path(parts.uri, &stripped);
            let cx = cx.with(parts);
            next.run(&cx, body).await
        })
    }
}

/// Removes `prefix` from a URI path when it matches as a whole path prefix.
///
/// `/res` matches `/res` and `/res/hello`, not `/resfoo`. The remainder is at
/// least `/`. Returns `None` when the path does not start with the prefix.
pub(crate) fn strip_path_prefix(path: &str, prefix: &str) -> Option<String> {
    if prefix.is_empty() || prefix == "/" {
        return Some(path.to_owned());
    }
    let rest = path.strip_prefix(prefix)?;
    if rest.is_empty() {
        Some("/".to_owned())
    } else if rest.starts_with('/') {
        Some(rest.to_owned())
    } else {
        None
    }
}

/// Replaces the path of `uri`, keeping the query string.
pub(crate) fn rewrite_path(uri: http::Uri, path: &str) -> http::Uri {
    let path_and_query = match uri.query() {
        Some(query) => format!("{path}?{query}"),
        None => path.to_owned(),
    };
    let mut parts = uri.into_parts();
    parts.path_and_query = Some(
        path_and_query
            .parse()
            .expect("stripped path is a valid URI path"),
    );
    http::Uri::from_parts(parts).expect("rewritten URI is valid")
}

/// Static URI prefix of a route path: the segments before the first `{`.
pub(crate) fn route_uri_prefix(path: &Path) -> &str {
    let path = path.as_str();
    if path.is_empty() {
        return "";
    }
    match path.find("/{") {
        Some(0) => "",
        Some(index) => &path[..index],
        None => path,
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;

    use http::StatusCode;
    use topcoat_core::context::Cx;

    use super::*;
    use crate::{
        Body, Method, RouteFn, RouteFuture, Router, request::Bytes, response::IntoResponse,
        to_bytes,
    };

    fn block_on<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(future)
    }

    fn body_bytes(response: crate::response::Response) -> Bytes {
        let (_, body) = response.into_parts();
        block_on(to_bytes(body, usize::MAX)).unwrap()
    }

    fn echo_path(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { uri(cx).to_string().into_response(cx) })
    }

    fn send(router: &Router, uri: &str) -> crate::response::Response {
        let request = http::Request::builder()
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        block_on(router.handle(request))
    }

    #[test]
    fn strip_path_prefix_requires_a_segment_boundary() {
        assert_eq!(
            strip_path_prefix("/res/hello", "/res").as_deref(),
            Some("/hello")
        );
        assert_eq!(strip_path_prefix("/res", "/res").as_deref(), Some("/"));
        assert_eq!(strip_path_prefix("/resfoo", "/res"), None);
        assert_eq!(strip_path_prefix("/other", "/res"), None);
    }

    #[test]
    fn route_uri_prefix_stops_at_the_first_parameter() {
        assert_eq!(route_uri_prefix(Path::new("/res/{*path}")), "/res");
        assert_eq!(route_uri_prefix(Path::new("/{*path}")), "");
        assert_eq!(route_uri_prefix(Path::new("/res")), "/res");
        assert_eq!(route_uri_prefix(Path::ROOT), "");
    }

    #[test]
    fn strip_prefix_rewrites_the_request_uri() {
        let router = Router::builder()
            .route(RouteFn::new(
                Method::GET,
                Path::new("/res/{*path}"),
                echo_path,
            ))
            .layer(StripPrefix::new("/res"))
            .build();

        let response = send(&router, "/res/hello.txt?cache=1");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(&body_bytes(response)[..], b"/hello.txt?cache=1");
    }

    #[test]
    fn strip_prefix_leaves_a_non_matching_path_unchanged() {
        let router = Router::builder()
            .route(RouteFn::new(
                Method::GET,
                Path::new("/legacy/{*rest}"),
                echo_path,
            ))
            .layer(StripPrefix::new("/res").at("/legacy"))
            .build();

        let response = send(&router, "/legacy/users/7");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(&body_bytes(response)[..], b"/legacy/users/7");
    }
}
