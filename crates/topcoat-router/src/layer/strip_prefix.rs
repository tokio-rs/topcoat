use std::borrow::Cow;

use topcoat_core::context::Cx;

use crate::{
    Body, IntoPath, Layer, LayerFuture, Next, Path, PathSegment,
    request::{parts, uri},
};

/// Removes a route pattern's matching prefix from the request URI.
///
/// Use this layer when a handler or mounted service expects paths relative to
/// its mount point. With a prefix of `/res`, a request for `/res/logo.svg`
/// reaches the handler as `/logo.svg`.
///
/// Register the layer with [`RouterBuilder::layer`](crate::RouterBuilder::layer).
/// By default, it wraps matched routes under the prefix passed to
/// [`new`](Self::new). Use [`at`](Self::at) to choose a different route scope.
///
/// # Example
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{Router, StripPrefixLayer, request::uri, route},
/// };
///
/// #[route(GET "/res/{*path}")]
/// async fn files(cx: &Cx) -> Result<String> {
///     // A request for `/res/logo.svg?version=2` returns `/logo.svg?version=2`.
///     Ok(uri(cx).to_string())
/// }
///
/// let router = Router::builder()
///     .route(files)
///     .layer(StripPrefixLayer::new("/res"))
///     .build();
/// ```
///
/// # Matching the prefix
///
/// The prefix uses [`Path`] syntax and matches whole URI segments:
///
/// - Static segments match literally. `/res` matches `/res/logo.svg`, but not
///   `/resources/logo.svg`.
/// - Parameters match one non-empty segment. `/tenants/{id}` removes `/tenants/42` from
///   `/tenants/42/logo.svg`.
/// - Groups take part in route scoping but consume no URI segments. `/(assets)/res` removes `/res`
///   from requests to routes in that group.
/// - A catch-all consumes the non-empty remaining path. `/res/{*path}` turns `/res/css/site.css`
///   into `/`.
///
/// The remaining path and query string keep their original encoding. When the
/// entire path is consumed, the new path is `/`. A prefix ending in `/` matches
/// only a URI ending at that slash. A prefix of `/` leaves the path unchanged.
/// If the URI does not match the prefix, the request passes through unchanged.
///
/// # Request handling
///
/// Route matching happens before this layer runs. Only the inner layers and
/// handler see the rewritten URI. Captured path parameters keep their matched
/// values. Response headers and generated URLs are unchanged.
#[derive(Debug, Clone)]
pub struct StripPrefixLayer {
    /// The route scope, including groups, whose matched routes this layer wraps.
    path: Cow<'static, Path>,
    /// The pattern used to consume the leading URI segments.
    prefix: Cow<'static, Path>,
}

impl StripPrefixLayer {
    /// Creates a layer that strips `prefix` from requests to routes under it.
    ///
    /// `prefix` sets both the route scope and the pattern to strip. Matching
    /// follows the [segment rules](Self#matching-the-prefix) above. Change the
    /// scope independently with [`at`](Self::at).
    ///
    /// # Example
    ///
    /// ```rust
    /// use topcoat::router::StripPrefixLayer;
    ///
    /// // Routes under `/tenants/{id}` see `/files/logo.svg` for a request to
    /// // `/tenants/42/files/logo.svg`.
    /// let layer = StripPrefixLayer::new("/tenants/{id}");
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `prefix` is a string that is not a well-formed route path.
    #[must_use]
    #[track_caller]
    pub fn new(prefix: impl IntoPath) -> Self {
        let path = prefix.into_path();
        let prefix = path.clone();
        Self { path, prefix }
    }

    /// Applies the layer to matched routes under `path`, keeping the stripping
    /// pattern passed to [`new`](Self::new).
    ///
    /// The scope selects handlers by their declared paths, including group
    /// segments and parameter names. The stripping pattern separately matches
    /// the request URI.
    ///
    /// # Example
    ///
    /// ```rust
    /// use topcoat::router::StripPrefixLayer;
    ///
    /// // Strip `/res` only for routes under `/(assets)/res`.
    /// let layer = StripPrefixLayer::new("/res").at("/(assets)/res");
    /// ```
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

impl Layer for StripPrefixLayer {
    fn path(&self) -> Option<&Path> {
        Some(&self.path)
    }

    fn handle<'a>(&'a self, cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            let Some(stripped) = strip_path_prefix(uri(cx).path(), &self.prefix) else {
                return next.run(cx, body).await;
            };
            let mut parts = parts(cx).clone();
            parts.uri = rewrite_path(parts.uri, stripped);
            let cx = cx.with(parts);
            next.run(&cx, body).await
        })
    }
}

/// Matches the prefix's route segments against the URI and returns the remainder.
///
/// Groups consume nothing, parameters consume one non-empty segment, and a
/// catch-all consumes the non-empty remainder. Static segments match literally.
/// Returns `/` when the path is consumed, or `None` when a segment does not match.
pub(crate) fn strip_path_prefix<'a>(path: &'a str, prefix: &Path) -> Option<&'a str> {
    let mut rest = path;
    for segment in prefix.segments() {
        match segment {
            PathSegment::Group(_) => {}
            PathSegment::Static("") => return (rest == "/").then_some("/"),
            PathSegment::Static(expected) => {
                rest = rest.strip_prefix('/')?.strip_prefix(expected)?;
                if !rest.is_empty() && !rest.starts_with('/') {
                    return None;
                }
            }
            PathSegment::Param(_) => {
                let body = rest.strip_prefix('/')?;
                let end = body.find('/').unwrap_or(body.len());
                if end == 0 {
                    return None;
                }
                rest = &body[end..];
            }
            PathSegment::CatchAll(_) => {
                return (!rest.strip_prefix('/')?.is_empty()).then_some("/");
            }
        }
    }
    Some(if rest.is_empty() { "/" } else { rest })
}

/// Replaces the URI path, preserving its query string, scheme, and authority.
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
            strip_path_prefix("/res/hello", Path::new("/res")),
            Some("/hello")
        );
        assert_eq!(strip_path_prefix("/res", Path::new("/res")), Some("/"));
        assert_eq!(strip_path_prefix("/resfoo", Path::new("/res")), None);
        assert_eq!(strip_path_prefix("/other", Path::new("/res")), None);
    }

    #[test]
    fn strip_path_prefix_matches_route_segments() {
        for (prefix, path, expected) in [
            ("/tenants/{id}", "/tenants/42/files", Some("/files")),
            ("/tenants/{id}", "/tenants/42", Some("/")),
            ("/tenants/{id}", "/tenants/42/", Some("/")),
            ("/tenants/{id}", "/tenants", None),
            ("/tenants/{id}", "/tenants/", None),
            ("/tenants/{id}", "/tenants//files", None),
            ("/{tenant}/files/{id}", "/acme/files/42/raw", Some("/raw")),
            ("/{tenant}/files/{id}", "/acme/files-old/42", None),
            ("/{tenant}/files/{id}", "/acme/files", None),
            ("/tenants/{id}", "/tenants/a%2Fb/a%20b", Some("/a%20b")),
            ("/(assets)/res", "/res/logo.svg", Some("/logo.svg")),
            ("/res/(assets)", "/res/logo.svg", Some("/logo.svg")),
            ("/(assets)", "/logo.svg", Some("/logo.svg")),
            ("/res/{*path}", "/res/css/site.css", Some("/")),
            ("/res/{*path}", "/res", None),
            ("/res/{*path}", "/res/", None),
            ("/res/", "/res/", Some("/")),
            ("/res/", "/res", None),
            ("/res/", "/res/file", None),
            ("/", "/", Some("/")),
            ("/", "/res/file", Some("/res/file")),
        ] {
            assert_eq!(
                strip_path_prefix(path, Path::new(prefix)),
                expected,
                "prefix {prefix:?}, path {path:?}"
            );
        }
    }

    #[test]
    fn strip_prefix_rewrites_grouped_and_parameterized_routes() {
        for (prefix, route, uri, expected) in [
            (
                "/(assets)/tenants/{id}",
                "/(assets)/tenants/{id}/{*path}",
                "https://example.com/tenants/a%2Fb/files/a%20b?cache=%2F",
                "https://example.com/files/a%20b?cache=%2F",
            ),
            (
                "/tenants/{id}",
                "/tenants/{id}",
                "/tenants/42?cache=1",
                "/?cache=1",
            ),
            (
                "/res/{*path}",
                "/res/{*path}",
                "/res/css/site.css?cache=1",
                "/?cache=1",
            ),
        ] {
            let router = Router::builder()
                .route(RouteFn::new(Method::GET, Path::new(route), echo_path))
                .layer(StripPrefixLayer::new(prefix))
                .build();

            let response = send(&router, uri);
            assert_eq!(response.status(), StatusCode::OK, "{prefix}");
            assert_eq!(&body_bytes(response)[..], expected.as_bytes(), "{prefix}");
        }
    }

    #[test]
    fn strip_prefix_can_be_scoped_to_a_different_route_pattern() {
        let router = Router::builder()
            .route(RouteFn::new(
                Method::GET,
                Path::new("/(assets)/tenants/{id}/{*path}"),
                echo_path,
            ))
            .layer(StripPrefixLayer::new("/tenants/{tenant}").at("/(assets)/tenants/{id}"))
            .build();

        let response = send(&router, "/tenants/42/logo.svg?cache=1");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(&body_bytes(response)[..], b"/logo.svg?cache=1");
    }

    #[test]
    fn strip_prefix_rewrites_the_request_uri() {
        let router = Router::builder()
            .route(RouteFn::new(
                Method::GET,
                Path::new("/res/{*path}"),
                echo_path,
            ))
            .layer(StripPrefixLayer::new("/res"))
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
            .layer(StripPrefixLayer::new("/res").at("/legacy"))
            .build();

        let response = send(&router, "/legacy/users/7");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(&body_bytes(response)[..], b"/legacy/users/7");
    }
}
