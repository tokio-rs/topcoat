use std::{
    future::{Future, poll_fn},
    panic::{AssertUnwindSafe, catch_unwind},
    pin::pin,
    sync::Arc,
    task::Poll,
};

use http::request::Parts;
use topcoat_core::{
    context::{AppContext, ContextValues, Cx, RequestContext, try_request_context, with_identity},
    error::Result,
};

use crate::{
    Body, Endpoint, EndpointIndex, Endpoints, Layer, Next, OriginLayer, RawPathParams, Route, RouteId,
    RouteIndex, RouterBuilder, Routes, Terminal, TrustedProxies,
    error::{REWRITE_LIMIT, RewriteError, RewriteLoopError, internal_server_response, respond},
    proxy::ClientIp,
    request::{OriginalParts, Request},
    response::{Response, ResponseHeaders, response_headers},
};

/// A finalized Topcoat routing table.
///
/// Build one with [`Router::builder`], register pages, layouts, layers, routes,
/// and app context values on the returned [`RouterBuilder`], then call
/// [`RouterBuilder::build`]. Most applications use the `topcoat` facade and
/// pass the finished router to `topcoat::start`.
///
/// # Examples
///
/// ```rust
/// # async fn example() -> topcoat::Result<()> {
/// use topcoat::router::{Router, RouterBuilderDiscoverExt};
///
/// let router = Router::builder().discover().build();
///
/// topcoat::start(router).await?;
/// # Ok(())
/// # }
/// ```
pub struct Router {
    /// The routing tables, shared with every request context this router
    /// creates.
    inner: Arc<RouterInner>,
}

impl Router {
    /// Creates an empty [`RouterBuilder`].
    #[must_use]
    pub fn builder() -> RouterBuilder {
        RouterBuilder::new()
    }

    /// Wraps finalized routing tables into a router.
    pub(crate) fn new(inner: RouterInner) -> Self {
        Self {
            inner: Arc::new(inner),
        }
    }

    /// Dispatches a request to the route registered for its path and method,
    /// producing a response.
    ///
    /// A route registered for the request's specific method wins over an
    /// any-method route at the same path. Returns `404 Not Found` when no
    /// route matches the path, or `405 Method Not Allowed` (with an `Allow`
    /// header) when the path matches but no route accepts the method. A panic
    /// while processing the request becomes a `500 Internal Server Error`
    /// response.
    pub async fn handle(&self, request: Request) -> Response {
        let mut future = pin!(self.handle_inner(request));

        poll_fn(|cx| {
            // The whole request and its context are discarded after a panic,
            // so no potentially inconsistent request-local state is reused.
            match catch_unwind(AssertUnwindSafe(|| future.as_mut().poll(cx))) {
                Ok(Poll::Ready(response)) => Poll::Ready(response),
                Ok(Poll::Pending) => Poll::Pending,
                Err(_) => Poll::Ready(internal_server_response()),
            }
        })
        .await
    }

    /// Handles one request inside the panic isolation boundary.
    async fn handle_inner(&self, request: Request) -> Response {
        let inner = &*self.inner;
        // The client's address comes from the headers the request arrived
        // with, which no rewrite changes, so it is resolved once.
        let client_ip = ClientIp(inner.trusted_proxies.resolve(&request));
        let (mut parts, mut body) = request.into_parts();
        let original = OriginalParts(parts.clone());
        let mut chain = RewriteChain::default();

        let (cx, result) = loop {
            // Every dispatch starts from a fresh context, so a rewrite drops
            // whatever the discarded dispatch registered or queued. Only the
            // values carried by rewrites come along, and the router's own
            // values are installed after them so they take precedence.
            let cx = Cx::new(Arc::clone(&inner.app_context))
                .with_many(chain.context.clone())
                .with_many((
                    Arc::clone(&self.inner),
                    ResponseHeaders::new(),
                    RawPathParams::default(),
                    original.clone(),
                    client_ip,
                    parts,
                ));
            let cx = match crate::request::initial_identity(&cx) {
                Ok(identity) => with_identity(cx, identity),
                Err(error) => break (cx, Err(error)),
            };

            let (cx, result) = self.dispatch(cx, body).await;

            // A rewrite bubbling out of the chain discards this dispatch,
            // response and request context both, and goes around again with
            // the new path; any other outcome ends the loop.
            let rewrite = match result {
                Ok(response) => break (cx, Ok(response)),
                Err(error) => match error.downcast::<RewriteError>() {
                    Ok(rewrite) => rewrite,
                    Err(failed) => break (cx, Err(failed.into_error())),
                },
            };
            (parts, body) = match chain.follow(crate::request::parts(&cx), rewrite) {
                Ok(request) => request.into_parts(),
                Err(error) => break (cx, Err(error)),
            };
        };
        let mut response = respond(&cx, result);
        response_headers(&cx).apply(response.headers_mut());

        // Compression runs outside every layer, so layers see uncompressed
        // bodies. The negotiation reads the request headers as the layers
        // left them.
        #[cfg(feature = "compression")]
        let response = inner
            .compression
            .compress(crate::request::headers(&cx), response)
            .await;

        response
    }

    /// Matches and runs one dispatch, retaining its context for the response.
    async fn dispatch(&self, cx: Cx, body: Body) -> (Cx, Result<Response>) {
        let inner = &*self.inner;
        let parts = crate::request::parts(&cx);
        let (cx, next) = match inner.endpoints.at(parts.uri.path()) {
            Some((endpoint_index, endpoint, params)) => {
                let path_params = RawPathParams::from_match(
                    endpoint.path(),
                    params.iter().map(|(_, value)| value),
                );
                let route_index = endpoint.get(&parts.method).or_else(|| endpoint.any());
                let next = match route_index {
                    Some(index) => Next::new(&[], Terminal::Route(&inner.routes[index])),
                    None => Next::new(&inner.always_layers, Terminal::MethodNotAllowed(endpoint)),
                };
                let matched = Matched {
                    endpoint: endpoint_index,
                    route: route_index,
                };
                (cx.with_many((matched, path_params)), next)
            }
            None => (cx, Next::new(&inner.always_layers, Terminal::NotFound)),
        };

        // The origin check precedes all route and pathless layers.
        let result = inner.origin.handle(&cx, body, next).await;
        (cx, result)
    }
}

/// The routing tables behind a [`Router`].
///
/// Every request carries a shared handle to these tables on its context, so
/// request-time accessors like [`endpoint`] resolve through the router that
/// dispatched the request.
pub(crate) struct RouterInner {
    /// The registered routes, indexed by the endpoints' method tables.
    pub(crate) routes: Routes,
    /// The registered endpoints and the matcher resolving a request URL to
    /// one of them.
    pub(crate) endpoints: Endpoints,
    /// The layers registered without a path, wrapping every request. A
    /// request matched to a route runs them as part of the route's own stack;
    /// this stack wraps requests that matched no route.
    pub(crate) always_layers: Box<[Arc<dyn Layer>]>,
    /// The values shared by every request, read back via
    /// [`app_context`](topcoat_core::context::app_context).
    pub(crate) app_context: Arc<AppContext>,
    /// The origin policy wrapping every request as the outermost layer.
    pub(crate) origin: OriginLayer,
    /// The proxies trusted to report the client's address.
    pub(crate) trusted_proxies: TrustedProxies,
    /// The compression applied to responses on their way out.
    #[cfg(feature = "compression")]
    pub(crate) compression: crate::Compression,
}

/// What a request remembers across the dispatches of a rewrite chain.
///
/// A request served in one dispatch leaves it empty. Each rewrite extends
/// it, and every dispatch after a rewrite starts its context from the
/// carried values.
#[derive(Default)]
struct RewriteChain {
    /// The paths the request was dispatched under so far, in order.
    visited: Vec<String>,
    /// The values rewrites carried into the later dispatches.
    context: RequestContext,
}

impl RewriteChain {
    /// Records the dispatch with `previous` as the one `rewrite` came out of
    /// and builds the request for the next dispatch, which keeps the headers,
    /// and the method unless the rewrite changes it, under the rewritten path
    /// and query.
    ///
    /// Fails when the request was already dispatched under the rewritten
    /// path or the chain grew past the rewrite limit.
    fn follow(&mut self, previous: &Parts, rewrite: RewriteError) -> Result<Request> {
        let rewrite = rewrite.into_parts();
        let target = rewrite.path_and_query;
        let uri = &previous.uri;
        let path = uri
            .path_and_query()
            .map_or(uri.path(), http::uri::PathAndQuery::as_str);
        self.visited.push(path.to_owned());
        if self.visited.iter().any(|path| path == target.as_str()) {
            return Err(RewriteLoopError::cycle(&self.visited, target.as_str()).into());
        }
        if self.visited.len() > REWRITE_LIMIT {
            return Err(RewriteLoopError::limit(&self.visited, target.as_str()).into());
        }

        let mut parts = previous.clone();
        let mut uri = std::mem::take(&mut parts.uri).into_parts();
        uri.path_and_query = Some(target);
        parts.uri = http::Uri::from_parts(uri)
            .expect("replacing the path of a valid request uri keeps it valid");
        if let Some(method) = rewrite.method {
            parts.method = method;
        }
        rewrite.context.install(&mut self.context);
        Ok(Request::from_parts(parts, rewrite.body))
    }
}

/// What a request was dispatched to, stored on its context.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Matched {
    /// The endpoint the request's path matched.
    endpoint: EndpointIndex,
    /// The route serving the request, or `None` when the path matched but no
    /// route accepts the method (a 405).
    route: Option<RouteIndex>,
}

/// Reads the router and dispatch record off a matched request's context.
fn try_matched(cx: &Cx) -> Option<(&RouterInner, Matched)> {
    let router = try_request_context::<Arc<RouterInner>>(cx)?;
    let matched = try_request_context::<Matched>(cx)?;
    Some((router, *matched))
}

/// Returns the endpoint the current request matched.
///
/// The endpoint's [`path`](Endpoint::path) is the pattern the request URL was
/// matched against rather than the URL itself, so parameters keep their
/// `{name}` form. Read the requested URL with [`uri`](crate::request::uri)
/// instead.
///
/// # Panics
///
/// Panics if the request matched no endpoint.
///
/// # Examples
///
/// ```rust
/// use topcoat::{context::Cx, router::endpoint};
///
/// fn log_line(cx: &Cx) -> String {
///     // The pattern, not the URL: `/users/{id}` for a request to `/users/42`.
///     format!("handled {}", endpoint(cx).path())
/// }
/// ```
#[must_use]
#[track_caller]
pub fn endpoint(cx: &Cx) -> &Endpoint {
    match try_endpoint(cx) {
        Some(endpoint) => endpoint,
        None => panic!("this request matched no endpoint"),
    }
}

/// Returns the endpoint the current request matched, or `None` if its path
/// matched none (a 404) or no router dispatched it.
///
/// See [`endpoint`] for what the match holds.
#[must_use]
pub fn try_endpoint(cx: &Cx) -> Option<&Endpoint> {
    let (router, matched) = try_matched(cx)?;
    Some(&router.endpoints[matched.endpoint])
}

/// Returns the route handling the current request.
///
/// # Panics
///
/// Panics if the request matched no route: either its path matched no
/// endpoint, or the endpoint holds no route for the request's method.
#[must_use]
#[track_caller]
pub fn route(cx: &Cx) -> &dyn Route {
    match try_route(cx) {
        Some(route) => route,
        None => panic!("this request matched no route"),
    }
}

/// Returns the route handling the current request, or `None` if none matched:
/// the path matched no endpoint (a 404), the endpoint holds no route for the
/// request's method (a 405), or no router dispatched the request.
#[must_use]
pub fn try_route(cx: &Cx) -> Option<&dyn Route> {
    let (router, matched) = try_matched(cx)?;
    Some(&router.routes[matched.route?])
}

/// Returns the endpoint serving the route registered under `id` on the router
/// the current request was dispatched through, or `None` if the context
/// carries no router or the router holds no route with that identity.
#[doc(hidden)]
#[must_use]
pub fn route_endpoint(cx: &Cx, id: RouteId) -> Option<&Endpoint> {
    let router = try_request_context::<Arc<RouterInner>>(cx)?;
    let index = router.routes.endpoint(id)?;
    Some(&router.endpoints[index])
}

/// Builds the request context of a request matched to an endpoint at `path`,
/// as the router assembles one, for tests elsewhere in this crate.
#[cfg(test)]
pub(crate) fn test_matched_cx(path: &crate::Path) -> Cx {
    use std::borrow::Cow;

    use crate::OriginPolicy;

    let mut endpoints = Endpoints::default();
    let endpoint = endpoints.push(Cow::Owned(path.as_str().to_owned()), Endpoint::new(path));
    let inner = RouterInner {
        routes: Routes::default(),
        endpoints,
        always_layers: Box::new([]),
        app_context: Arc::new(AppContext::new()),
        origin: OriginLayer::new(OriginPolicy::new()),
        trusted_proxies: TrustedProxies::new(),
        #[cfg(feature = "compression")]
        compression: crate::Compression::new(),
    };
    Cx::new(Arc::clone(&inner.app_context)).with_many((
        Arc::new(inner),
        Matched {
            endpoint,
            route: None,
        },
    ))
}

#[cfg(test)]
mod tests {
    use std::{
        borrow::Cow,
        future::Future,
        sync::{Arc, Mutex, OnceLock},
    };

    use http::{HeaderMap, StatusCode};
    use topcoat::view::{DynViewPart, HtmlWriter, NodeViewParts, PartsWriter, view};
    use topcoat_core::{
        context::{Cx, app_context, request_context},
        error::Result,
    };
    use topcoat_view::{BoxView, ViewExt, internal::MoveView};

    use super::*;
    use crate::{
        Body, HrefTarget, LayerFn, LayerFuture, LayoutFn, Method, Methods, OriginPolicy, PageFn,
        Path, RemoteAddr, Route, RouteFn, RouteFuture, Slot, TrailingSlash,
        error::rewrite,
        raw_path_params,
        request::{Bytes, client_ip, method, original_method, original_uri, uri},
        response::IntoResponse,
        to_bytes,
    };

    // -- Test helpers --

    fn block_on<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(future)
    }

    fn path(s: &'static str) -> Cow<'static, Path> {
        Cow::Borrowed(Path::new(s))
    }

    /// Builds a request with an empty body for the given method and path.
    fn request(method: Method, path: &str) -> Request {
        http::Request::builder()
            .method(method)
            .uri(path)
            .body(Body::empty())
            .unwrap()
    }

    /// Dispatches a request through the router and reads the full response.
    fn send(router: &Router, method: Method, path: &str) -> (StatusCode, HeaderMap, Bytes) {
        let response = block_on(router.handle(request(method, path)));
        let (parts, body) = response.into_parts();
        let bytes = block_on(to_bytes(body, usize::MAX)).unwrap();
        (parts.status, parts.headers, bytes)
    }

    // A handful of plain handler functions, since `Route`/`Layer` are backed by
    // `fn` pointers and cannot capture state.

    fn say_route(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { "route".into_response(cx) })
    }

    fn say_posted(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { "posted".into_response(cx) })
    }

    fn panic_route(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { panic!("request handler panicked") })
    }

    fn panic_before_future_route(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        panic!("request handler panicked before returning its future");
    }

    /// Echoes the captured path params as `key=value` pairs joined by `&`.
    fn echo_params(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move {
            raw_path_params(cx)
                .map(|(key, value)| format!("{key}={}", value.as_str()))
                .collect::<Vec<_>>()
                .join("&")
                .into_response(cx)
        })
    }

    /// Echoes the path of the endpoint the request matched.
    fn echo_endpoint_path(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { endpoint(cx).path().to_string().into_response(cx) })
    }

    /// Echoes the identity of the route handling the request.
    fn echo_route_id(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { format!("{:?}", route(cx).id()).into_response(cx) })
    }

    /// Reads a registered app-context greeting and returns it as the body.
    struct Greeting(&'static str);

    fn say_greeting(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { app_context::<Greeting>(cx).0.into_response(cx) })
    }

    /// Reads the registered base URL and returns it as the body.
    fn say_base_url(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move {
            topcoat_core::base_url::base_url(cx)
                .as_str()
                .to_owned()
                .into_response(cx)
        })
    }

    // Layers that record their label in a shared trace before continuing, so a
    // test can observe the order layers run in.
    type Trace = Mutex<Vec<&'static str>>;

    fn trace_always<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            app_context::<Arc<Trace>>(cx).lock().unwrap().push("always");
            next.run(cx, body).await
        })
    }

    fn trace_root<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            app_context::<Arc<Trace>>(cx).lock().unwrap().push("root");
            next.run(cx, body).await
        })
    }

    fn trace_admin<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            app_context::<Arc<Trace>>(cx).lock().unwrap().push("admin");
            next.run(cx, body).await
        })
    }

    fn trace_auth<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            app_context::<Arc<Trace>>(cx).lock().unwrap().push("auth");
            next.run(cx, body).await
        })
    }

    // Page and layout render functions for the rendering tests.
    fn render_page(cx: &Cx, _body: Body) -> BoxView<'static> {
        view! { cx => "page" }.boxed()
    }

    /// A view part that panics when it renders, so the router's panic
    /// handling during rendering is observable.
    #[derive(Debug, Clone)]
    struct Panicking;

    impl NodeViewParts for Panicking {
        fn into_view_parts(self, _cx: &Cx, parts: &mut PartsWriter<'_>) {
            parts.push_dyn(Box::new(self));
        }
    }

    impl DynViewPart for Panicking {
        fn render(&self, _cx: &Cx, _w: &mut HtmlWriter<'_, '_>) {
            panic!("view rendering panicked");
        }
    }

    fn render_panicking_page(cx: &Cx, _body: Body) -> BoxView<'static> {
        view! { cx => (Panicking) }.boxed()
    }

    /// Wraps the child content in `R[ ... ]` so layout nesting is observable.
    fn layout_root<'a>(cx: &Cx, slot: Slot<'a>) -> BoxView<'a> {
        view! {
            cx =>
            "R["
            (slot)
            "]"
        }
        .boxed()
    }

    /// Wraps the child content in `A[ ... ]`.
    fn layout_admin<'a>(cx: &Cx, slot: Slot<'a>) -> BoxView<'a> {
        view! {
            cx =>
            "A["
            (slot)
            "]"
        }
        .boxed()
    }

    // -- Router::handle: dispatch --

    #[test]
    fn routes_to_the_matching_method() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .route(RouteFn::new(Method::POST, path("/x"), say_posted))
            .build();

        let (status, _, body) = send(&router, Method::GET, "/x");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"route");

        let (status, _, body) = send(&router, Method::POST, "/x");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"posted");
    }

    #[test]
    fn unmatched_path_is_not_found() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .build();
        let (status, _, _) = send(&router, Method::GET, "/missing");
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn matched_path_wrong_method_is_method_not_allowed() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .route(RouteFn::new(Method::POST, path("/x"), say_posted))
            .build();
        let (status, headers, _) = send(&router, Method::DELETE, "/x");
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);

        // The `Allow` header lists the supported methods, including the `HEAD`
        // aliased onto `GET`.
        let allow = headers.get(http::header::ALLOW).unwrap().to_str().unwrap();
        assert!(allow.contains("GET"), "{allow:?}");
        assert!(allow.contains("POST"), "{allow:?}");
        assert!(allow.contains("HEAD"), "{allow:?}");
    }

    #[test]
    fn head_is_aliased_to_get() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .build();
        let (status, _, body) = send(&router, Method::HEAD, "/x");
        assert_eq!(status, StatusCode::OK);
        // The `GET` handler runs for a `HEAD` request.
        assert_eq!(&body[..], b"route");
    }

    #[test]
    fn captures_and_decodes_path_params() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/users/{id}"), echo_params))
            .build();

        let (_, _, body) = send(&router, Method::GET, "/users/42");
        assert_eq!(&body[..], b"id=42");

        // Percent-encoded values are decoded.
        let (_, _, body) = send(&router, Method::GET, "/users/a%20b");
        assert_eq!(&body[..], b"id=a b");
    }

    #[test]
    fn captures_catch_all_params() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(
                Method::GET,
                path("/files/{*rest}"),
                echo_params,
            ))
            .build();
        // The raw catch-all keeps the encoded remainder, slashes included.
        let (_, _, body) = send(&router, Method::GET, "/files/a%2Fb/c%20d");
        assert_eq!(&body[..], b"rest=a%2Fb/c%20d");
    }

    #[test]
    fn exposes_the_matched_endpoint_path() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(
                Method::GET,
                path("/users/{id}"),
                echo_endpoint_path,
            ))
            .route(RouteFn::new(
                Method::GET,
                path("/files/{*rest}"),
                echo_endpoint_path,
            ))
            .build();

        // The pattern the endpoint serves, not the requested URL.
        let (_, _, body) = send(&router, Method::GET, "/users/42");
        assert_eq!(&body[..], b"/users/{id}");

        let (_, _, body) = send(&router, Method::GET, "/files/a/b");
        assert_eq!(&body[..], b"/files/{*rest}");
    }

    #[test]
    fn the_endpoint_path_drops_group_segments() {
        // Groups bind layouts and layers at build time and are not part of the
        // URL, so routes that differ only in them agree on the path they share.
        let router = RouterBuilder::new()
            .route(RouteFn::new(
                Method::GET,
                path("/(a)/x"),
                echo_endpoint_path,
            ))
            .route(RouteFn::new(
                Method::POST,
                path("/(b)/x"),
                echo_endpoint_path,
            ))
            .build();

        for method in [Method::GET, Method::POST] {
            let (_, _, body) = send(&router, method, "/x");
            assert_eq!(&body[..], b"/x");
        }
    }

    #[test]
    fn handler_panics_become_internal_server_errors() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/panic"), panic_route))
            .route(RouteFn::new(
                Method::GET,
                path("/early-panic"),
                panic_before_future_route,
            ))
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .build();

        let (status, _, body) = send(&router, Method::GET, "/panic");
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(&body[..], b"internal server error");

        let (status, _, body) = send(&router, Method::GET, "/early-panic");
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(&body[..], b"internal server error");

        let (status, _, body) = send(&router, Method::GET, "/x");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"route");
    }

    // -- Router::handle: rewrites --

    // Rewriting handlers, one per target, since routes are plain `fn`
    // pointers.

    fn rewrite_to_x(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { Err(rewrite("/x", Body::empty()).into()) })
    }

    fn render_rewriting_page(_cx: &Cx, _body: Body) -> BoxView<'static> {
        Box::pin(MoveView::new(async move {
            Err(rewrite("/x", Body::empty()).into())
        }))
    }

    fn layout_rewrites<'a>(_cx: &Cx, _slot: Slot<'a>) -> BoxView<'a> {
        Box::pin(MoveView::new(async move {
            Err(rewrite("/x", Body::empty()).into())
        }))
    }

    fn rewrite_to_missing(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { Err(rewrite("/missing", Body::empty()).into()) })
    }

    fn rewrite_with_body(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { Err(rewrite("/echo-body", "carried").into()) })
    }

    fn rewrite_to_new_query(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { Err(rewrite("/new?q=2", Body::empty()).into()) })
    }

    fn rewrite_to_cycle_a(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { Err(rewrite("/cycle/a", Body::empty()).into()) })
    }

    fn rewrite_to_cycle_b(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { Err(rewrite("/cycle/b", Body::empty()).into()) })
    }

    /// Rewrites `/step/{n}` to `/step/{n + 1}`, a chain that never repeats a
    /// path.
    fn rewrite_to_next_step(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move {
            let step: usize = raw_path_params(cx)
                .next()
                .unwrap()
                .1
                .as_str()
                .parse()
                .unwrap();
            Err(rewrite(format!("/step/{}", step + 1), Body::empty()).into())
        })
    }

    /// Echoes the request body back as the response.
    fn echo_body(cx: &Cx, body: Body) -> RouteFuture<'_> {
        Box::pin(async move {
            let bytes = to_bytes(body, usize::MAX).await?;
            String::from_utf8_lossy(&bytes)
                .into_owned()
                .into_response(cx)
        })
    }

    /// Echoes the dispatched and original URIs, separated by a space.
    fn echo_uris(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { format!("{} {}", uri(cx), original_uri(cx)).into_response(cx) })
    }

    fn rewrite_to_x_as_get(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { Err(rewrite("/x", Body::empty()).method(Method::GET).into()) })
    }

    /// Echoes the dispatched and original methods, separated by a space.
    fn echo_methods(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { format!("{} {}", method(cx), original_method(cx)).into_response(cx) })
    }

    /// A value a rewrite hands to the handler at its target.
    struct Carried(&'static str);

    /// Rewrites to `/carried`, handing over a value.
    fn rewrite_with_value(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move {
            Err(rewrite("/carried", Body::empty())
                .with(Carried("handed over"))
                .into())
        })
    }

    /// Rewrites on to `/carried-again` without carrying another value.
    fn rewrite_onward(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { Err(rewrite("/carried-again", Body::empty()).into()) })
    }

    /// Echoes the carried value, or `none` when the context holds none.
    fn echo_carried(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move {
            try_request_context::<Carried>(cx)
                .map_or("none", |carried| carried.0)
                .into_response(cx)
        })
    }

    #[test]
    fn a_rewrite_serves_the_new_path() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/old"), rewrite_to_x))
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .build();

        let (status, _, body) = send(&router, Method::GET, "/old");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"route");
    }

    #[test]
    fn a_page_without_a_layout_can_rewrite() {
        let router = RouterBuilder::new()
            .page(PageFn::new(
                Method::GET,
                path("/old"),
                render_rewriting_page,
            ))
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .build();

        let (status, _, body) = send(&router, Method::GET, "/old");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"route");
    }

    #[test]
    fn a_page_wrapped_by_a_layout_can_rewrite() {
        let router = RouterBuilder::new()
            .page(PageFn::new(
                Method::GET,
                path("/old"),
                render_rewriting_page,
            ))
            .layout(LayoutFn::new(path("/"), layout_root))
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .build();

        let (status, _, body) = send(&router, Method::GET, "/old");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"route");
    }

    #[test]
    fn a_layout_can_rewrite() {
        let router = RouterBuilder::new()
            .page(PageFn::new(Method::GET, path("/old"), render_page))
            .layout(LayoutFn::new(path("/"), layout_rewrites))
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .build();

        let (status, _, body) = send(&router, Method::GET, "/old");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"route");
    }

    #[test]
    fn a_rewrite_keeps_the_request_method() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::POST, path("/old"), rewrite_to_x))
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .build();

        // The rewritten dispatch is still a `POST`, which `/x` does not serve.
        let (status, _, _) = send(&router, Method::POST, "/old");
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }

    #[test]
    fn a_rewrite_can_change_the_method_and_keeps_the_original_readable() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(
                Method::POST,
                path("/old"),
                rewrite_to_x_as_get,
            ))
            .route(RouteFn::new(Method::GET, path("/x"), echo_methods))
            .build();

        let (status, _, body) = send(&router, Method::POST, "/old");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"GET POST");
    }

    #[test]
    fn a_rewrite_carries_its_values_through_the_rest_of_the_chain() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/old"), rewrite_with_value))
            .route(RouteFn::new(Method::GET, path("/carried"), rewrite_onward))
            .route(RouteFn::new(
                Method::GET,
                path("/carried-again"),
                echo_carried,
            ))
            .build();

        // The value survives the rewrite that carried it and the plain
        // rewrite after it.
        let (_, _, body) = send(&router, Method::GET, "/old");
        assert_eq!(&body[..], b"handed over");

        // A dispatch that was not reached through the carrying rewrite
        // starts from an empty context.
        let (_, _, body) = send(&router, Method::GET, "/carried-again");
        assert_eq!(&body[..], b"none");
    }

    #[test]
    fn later_rewrites_replace_values_of_the_same_type_and_keep_other_values() {
        fn start(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
            Box::pin(async {
                Err(rewrite("/replace", Body::empty())
                    .with(Carried("superseded"))
                    .with(Carried("first"))
                    .with(42u32)
                    .into())
            })
        }

        fn replace(cx: &Cx, _body: Body) -> RouteFuture<'_> {
            Box::pin(async move {
                assert_eq!(request_context::<Carried>(cx).0, "first");
                Err(rewrite("/inspect", Body::empty())
                    .with(Carried("second"))
                    .into())
            })
        }

        fn inspect(cx: &Cx, _body: Body) -> RouteFuture<'_> {
            Box::pin(async move {
                assert_eq!(*request_context::<u32>(cx), 42);
                request_context::<Carried>(cx).0.into_response(cx)
            })
        }

        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/start"), start))
            .route(RouteFn::new(Method::GET, path("/replace"), replace))
            .route(RouteFn::new(Method::GET, path("/inspect"), inspect))
            .build();

        let (status, _, body) = send(&router, Method::GET, "/start");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"second");
    }

    #[test]
    fn carrying_values_does_not_carry_dispatch_state() {
        use topcoat_core::context::{CxId, identity, memoize_cache};

        fn cached_id(cx: &Cx) -> CxId {
            *memoize_cache(cx).memoize(cx, (), (), |cx, ()| cx.id())
        }

        fn start(cx: &Cx, _body: Body) -> RouteFuture<'_> {
            Box::pin(async move {
                let cx = cx.with(Carried("local")).keyed("local");
                response_headers(&cx).append(
                    http::HeaderName::from_static("x-abandoned"),
                    http::HeaderValue::from_static("yes"),
                );
                Err(rewrite("/inspect", Body::empty())
                    .with(cached_id(&cx))
                    .into())
            })
        }

        fn inspect(cx: &Cx, _body: Body) -> RouteFuture<'_> {
            Box::pin(async move {
                assert_ne!(*request_context::<CxId>(cx), cx.id());
                assert_eq!(cached_id(cx), cx.id());
                assert!(try_request_context::<Carried>(cx).is_none());
                assert_eq!(identity(cx), crate::request::initial_identity(cx)?);
                "fresh".into_response(cx)
            })
        }

        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/start"), start))
            .route(RouteFn::new(Method::GET, path("/inspect"), inspect))
            .build();

        let (status, headers, body) = send(&router, Method::GET, "/start");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"fresh");
        assert!(!headers.contains_key("x-abandoned"));
    }

    #[test]
    fn a_rewrite_carries_its_body() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/form"), rewrite_with_body))
            .route(RouteFn::new(Method::GET, path("/echo-body"), echo_body))
            .build();

        let (status, _, body) = send(&router, Method::GET, "/form");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"carried");
    }

    #[test]
    fn a_rewrite_swaps_the_query_and_keeps_the_original_uri_readable() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(
                Method::GET,
                path("/old"),
                rewrite_to_new_query,
            ))
            .route(RouteFn::new(Method::GET, path("/new"), echo_uris))
            .build();

        let (_, _, body) = send(&router, Method::GET, "/old?client=1");
        assert_eq!(&body[..], b"/new?q=2 /old?client=1");
    }

    #[test]
    fn original_uri_is_the_request_uri_without_a_rewrite() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/new"), echo_uris))
            .build();

        let (_, _, body) = send(&router, Method::GET, "/new?q=2");
        assert_eq!(&body[..], b"/new?q=2 /new?q=2");
    }

    #[test]
    fn a_rewrite_to_an_unmatched_path_is_not_found() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/old"), rewrite_to_missing))
            .build();

        let (status, _, _) = send(&router, Method::GET, "/old");
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[test]
    fn a_rewrite_reruns_pathless_layers() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/old"), rewrite_to_x))
                .route(RouteFn::new(Method::GET, path("/x"), say_route))
                .layer(LayerFn::new(None::<&Path>, trace_always)),
        );

        let (status, _, _) = send(&router, Method::GET, "/old");
        assert_eq!(status, StatusCode::OK);
        // Once around the abandoned dispatch, once around the rewritten one.
        assert_eq!(*trace.lock().unwrap(), vec!["always", "always"]);
    }

    #[test]
    fn a_rewrite_cycle_is_an_internal_server_error() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(
                Method::GET,
                path("/cycle/a"),
                rewrite_to_cycle_b,
            ))
            .route(RouteFn::new(
                Method::GET,
                path("/cycle/b"),
                rewrite_to_cycle_a,
            ))
            .build();

        let (status, _, body) = send(&router, Method::GET, "/cycle/a");
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        // The chain never leaks to the client.
        assert_eq!(&body[..], b"internal server error");
    }

    #[test]
    fn a_runaway_rewrite_chain_stops_at_the_limit() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(
                Method::GET,
                path("/step/{n}"),
                rewrite_to_next_step,
            ))
            .build();

        let (status, _, body) = send(&router, Method::GET, "/step/0");
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(&body[..], b"internal server error");
    }

    // -- endpoint / route accessors --

    #[test]
    fn reads_the_matched_endpoint() {
        let cx = test_matched_cx(Path::new("/users/{id}"));
        assert_eq!(endpoint(&cx).path(), Path::new("/users/{id}"));
    }

    #[test]
    #[should_panic(expected = "matched no endpoint")]
    fn endpoint_panics_without_a_match() {
        let _ = endpoint(&Cx::default());
    }

    #[test]
    fn exposes_the_matched_route() {
        let route = RouteFn::new(Method::GET, path("/x"), echo_route_id);
        let expected = format!("{:?}", route.id());
        let router = RouterBuilder::new().route(route).build();

        let (status, _, body) = send(&router, Method::GET, "/x");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], expected.as_bytes());
    }

    #[test]
    #[should_panic(expected = "matched no route")]
    fn route_panics_without_a_matched_route() {
        // The context of a 405: an endpoint matched, but no route did.
        let cx = test_matched_cx(Path::new("/x"));
        let _ = route(&cx);
    }

    #[test]
    fn try_accessors_report_a_partial_match() {
        // The context of a 405: the endpoint is readable, the route is not.
        let cx = test_matched_cx(Path::new("/x"));
        assert_eq!(try_endpoint(&cx).unwrap().path(), Path::new("/x"));
        assert!(try_route(&cx).is_none());
    }

    #[test]
    fn try_accessors_report_no_match() {
        let cx = Cx::default();
        assert!(try_endpoint(&cx).is_none());
        assert!(try_route(&cx).is_none());
    }

    /// The route the href test resolves, shared with its handler through a
    /// static since handlers are plain `fn` pointers.
    static HREF_ROUTE: OnceLock<RouteFn> = OnceLock::new();

    fn href_route() -> RouteFn {
        HREF_ROUTE
            .get_or_init(|| RouteFn::new(Method::GET, path("/(auth)/users/{id}"), echo_href_path))
            .clone()
    }

    fn echo_href_path(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move {
            HrefTarget::path(&href_route(), cx)
                .to_string()
                .into_response(cx)
        })
    }

    #[test]
    fn href_target_resolves_the_served_url_path() {
        let router = RouterBuilder::new().route(href_route()).build();
        let (_, _, body) = send(&router, Method::GET, "/users/7");
        // The endpoint's URL path: groups stripped, params in `{name}` form.
        assert_eq!(&body[..], b"/users/{id}");
    }

    // -- Router::handle: origin policy --

    #[test]
    fn untrusted_cross_origin_requests_are_forbidden() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::POST, path("/x"), say_posted))
            .build();
        let request = http::Request::builder()
            .method(Method::POST)
            .uri("/x")
            .header("sec-fetch-site", "cross-site")
            .body(Body::empty())
            .unwrap();
        let response = block_on(router.handle(request));
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn origin_verification_wraps_unmatched_requests() {
        // The origin layer runs before the 404 resolves, so an untrusted
        // cross-origin request learns nothing about which paths exist.
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::POST, path("/x"), say_posted))
            .build();
        let request = http::Request::builder()
            .method(Method::POST)
            .uri("/missing")
            .header("sec-fetch-site", "cross-site")
            .body(Body::empty())
            .unwrap();
        let response = block_on(router.handle(request));
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[test]
    fn exempt_paths_skip_origin_verification() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::POST, path("/x"), say_posted))
            .origin_policy(OriginPolicy::new().exempt_paths(["/x"]))
            .build();
        let request = http::Request::builder()
            .method(Method::POST)
            .uri("/x")
            .header("sec-fetch-site", "cross-site")
            .body(Body::empty())
            .unwrap();
        let response = block_on(router.handle(request));
        assert_eq!(response.status(), StatusCode::OK);
    }

    // -- Router::handle: client address --

    /// Echoes the client address, or `none`.
    fn echo_client_ip(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move {
            client_ip(cx)
                .map_or_else(|| "none".to_owned(), |ip| ip.to_string())
                .into_response(cx)
        })
    }

    /// Dispatches a request that arrived from `remote` with an
    /// `X-Forwarded-For` header, reading the body.
    fn send_forwarded(router: &Router, remote: &str, forwarded_for: &str) -> Bytes {
        let request = http::Request::builder()
            .method(Method::GET)
            .uri("/ip")
            .header("x-forwarded-for", forwarded_for)
            .extension(RemoteAddr(remote.parse().unwrap()))
            .body(Body::empty())
            .unwrap();
        let response = block_on(router.handle(request));
        block_on(to_bytes(response.into_body(), usize::MAX)).unwrap()
    }

    #[test]
    fn the_client_ip_is_the_peer_without_trusted_proxies() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/ip"), echo_client_ip))
            .build();
        let body = send_forwarded(&router, "10.0.0.1:4242", "198.51.100.1");
        assert_eq!(&body[..], b"10.0.0.1");

        // A request without a peer address has no client address either.
        let (_, _, body) = send(&router, Method::GET, "/ip");
        assert_eq!(&body[..], b"none");
    }

    /// A pathless layer that hands the chain the request without its
    /// forwarding header, as middleware rewriting headers would.
    fn strip_forwarded<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        let mut parts = request_context::<http::request::Parts>(cx).clone();
        parts.headers.remove("x-forwarded-for");
        let cx = cx.with(parts);
        Box::pin(async move { next.run(&cx, body).await })
    }

    #[test]
    fn the_client_ip_is_resolved_as_the_request_arrives() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/ip"), echo_client_ip))
            .layer(LayerFn::new(None::<&Path>, strip_forwarded))
            .trusted_proxies(TrustedProxies::new().networks(["10.0.0.0/8"]))
            .build();
        // The layer's edit to the headers changes nothing about the client.
        let body = send_forwarded(&router, "10.0.0.1:4242", "198.51.100.1");
        assert_eq!(&body[..], b"198.51.100.1");
    }

    #[test]
    fn the_client_ip_resolves_through_trusted_proxies() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/ip"), echo_client_ip))
            .trusted_proxies(TrustedProxies::new().networks(["10.0.0.0/8"]))
            .build();
        let body = send_forwarded(&router, "10.0.0.1:4242", "198.51.100.1");
        assert_eq!(&body[..], b"198.51.100.1");

        // The header of a request from an untrusted peer is not read.
        let body = send_forwarded(&router, "203.0.113.9:4242", "198.51.100.1");
        assert_eq!(&body[..], b"203.0.113.9");
    }

    // -- Router::handle: method sets --

    fn say_any(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { "any".into_response(cx) })
    }

    #[test]
    fn any_method_route_serves_every_method() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Methods::Any, path("/x"), say_any))
            .build();

        let purge = Method::from_bytes(b"PURGE").unwrap();
        for method in [Method::GET, Method::POST, Method::DELETE, purge] {
            let (status, _, body) = send(&router, method, "/x");
            assert_eq!(status, StatusCode::OK);
            assert_eq!(&body[..], b"any");
        }
    }

    #[test]
    fn specific_method_route_wins_over_any() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .route(RouteFn::new(Methods::Any, path("/x"), say_any))
            .build();

        let (_, _, body) = send(&router, Method::GET, "/x");
        assert_eq!(&body[..], b"route");
        let (_, _, body) = send(&router, Method::POST, "/x");
        assert_eq!(&body[..], b"any");
    }

    #[test]
    fn multi_method_route_serves_each_of_its_methods() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(
                &[Method::GET, Method::POST],
                path("/form"),
                say_route,
            ))
            .build();

        let (status, _, _) = send(&router, Method::GET, "/form");
        assert_eq!(status, StatusCode::OK);
        let (status, _, _) = send(&router, Method::POST, "/form");
        assert_eq!(status, StatusCode::OK);

        // Methods outside the set still resolve to a 405.
        let (status, _, _) = send(&router, Method::DELETE, "/form");
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }

    #[test]
    fn head_falls_back_to_an_any_method_route() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Methods::Any, path("/x"), say_any))
            .build();
        let (status, _, body) = send(&router, Method::HEAD, "/x");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"any");
    }

    #[test]
    fn app_context_is_available_to_handlers() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/hi"), say_greeting))
            .app_context(Greeting("hello"))
            .build();
        let (status, _, body) = send(&router, Method::GET, "/hi");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"hello");
    }

    #[test]
    fn base_url_is_available_to_handlers() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/x"), say_base_url))
            .base_url("https://example.com")
            .build();
        let (status, _, body) = send(&router, Method::GET, "/x");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"https://example.com");
    }

    // -- Router::handle: layers --

    fn trace_router(builder: RouterBuilder) -> (Router, Arc<Trace>) {
        let trace: Arc<Trace> = Arc::new(Mutex::new(Vec::new()));
        let router = builder.app_context(trace.clone()).build();
        (router, trace)
    }

    #[test]
    fn layers_run_outermost_first_by_path_specificity() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/admin/x"), say_route))
                .layer(LayerFn::new(Some(path("/admin")), trace_admin))
                .layer(LayerFn::new(Some(path("/")), trace_root))
                .layer(LayerFn::new(None::<&Path>, trace_always)),
        );

        let (status, _, body) = send(&router, Method::GET, "/admin/x");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"route");
        // The pathless layer (least specific of all) wraps the root layer,
        // which wraps the admin layer.
        assert_eq!(*trace.lock().unwrap(), vec!["always", "root", "admin"]);
    }

    #[test]
    fn layers_only_wrap_routes_under_their_path() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/admin/x"), say_route))
                .route(RouteFn::new(Method::GET, path("/public"), say_route))
                .layer(LayerFn::new(Some(path("/admin")), trace_admin)),
        );

        send(&router, Method::GET, "/public");
        assert!(trace.lock().unwrap().is_empty());

        send(&router, Method::GET, "/admin/x");
        assert_eq!(*trace.lock().unwrap(), vec!["admin"]);
    }

    #[test]
    fn pathless_layers_wrap_not_found_responses() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/admin/x"), say_route))
                .layer(LayerFn::new(None::<&Path>, trace_always)),
        );

        let (status, _, _) = send(&router, Method::GET, "/missing");
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(*trace.lock().unwrap(), vec!["always"]);
    }

    #[test]
    fn path_layers_do_not_wrap_not_found_responses() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/admin/x"), say_route))
                .layer(LayerFn::new(Some(path("/")), trace_root))
                .layer(LayerFn::new(Some(path("/admin")), trace_admin)),
        );

        // No route means no path layers, the root path included.
        let (status, _, _) = send(&router, Method::GET, "/admin/missing");
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(trace.lock().unwrap().is_empty());
    }

    /// A pathless layer that replaces any error coming back up the chain,
    /// standing in for a site-wide custom error page.
    fn replace_errors<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            match next.run(cx, body).await {
                Ok(response) => Ok(response),
                Err(_) => "replaced".into_response(cx),
            }
        })
    }

    #[test]
    fn a_pathless_layer_can_replace_a_not_found_response() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .layer(LayerFn::new(None::<&Path>, replace_errors))
            .build();

        let (status, _, body) = send(&router, Method::GET, "/missing");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"replaced");
    }

    /// A pathless layer that defers a header onto whatever response the
    /// request ends with, before it knows whether the chain will succeed.
    fn tag_response<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            response_headers(cx).append(
                http::HeaderName::from_static("x-tag"),
                http::HeaderValue::from_static("layer"),
            );
            next.run(cx, body).await
        })
    }

    #[test]
    fn deferred_headers_land_on_a_success_response() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .layer(LayerFn::new(None::<&Path>, tag_response))
            .build();

        let (status, headers, _) = send(&router, Method::GET, "/x");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(headers.get("x-tag").unwrap(), "layer");
    }

    /// The error's response is built after every layer has returned, which is
    /// too late for a layer to set a header on it directly.
    #[test]
    fn deferred_headers_land_on_an_error_response() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .layer(LayerFn::new(None::<&Path>, tag_response))
            .build();

        let (status, headers, _) = send(&router, Method::GET, "/missing");
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(headers.get("x-tag").unwrap(), "layer");
    }

    #[test]
    fn pathless_layers_wrap_method_not_allowed_responses() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/x"), say_route))
                .layer(LayerFn::new(None::<&Path>, trace_always)),
        );
        let (status, _, _) = send(&router, Method::POST, "/x");
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(*trace.lock().unwrap(), vec!["always"]);
    }

    #[test]
    fn dispatch_ignores_query_strings() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/admin/x"), say_route))
                .layer(LayerFn::new(Some(path("/admin")), trace_admin)),
        );

        let (status, _, _) = send(&router, Method::GET, "/admin/x?tab=users");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(*trace.lock().unwrap(), vec!["admin"]);
    }

    #[test]
    fn layers_wrap_percent_encoded_param_urls() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/admin/{id}"), say_route))
                .layer(LayerFn::new(Some(path("/admin")), trace_admin)),
        );
        let (status, _, _) = send(&router, Method::GET, "/admin/a%20b");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(*trace.lock().unwrap(), vec!["admin"]);
    }

    #[test]
    fn layers_wrap_catch_all_routes() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/admin/{*rest}"), say_route))
                .layer(LayerFn::new(Some(path("/admin")), trace_admin)),
        );
        let (status, _, _) = send(&router, Method::GET, "/admin/a/b/c");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(*trace.lock().unwrap(), vec!["admin"]);
    }

    #[test]
    fn group_layers_wrap_routes_at_their_stripped_url() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(
                    Method::GET,
                    path("/(auth)/dashboard"),
                    say_route,
                ))
                .layer(LayerFn::new(Some(path("/(auth)")), trace_auth)),
        );

        // The route serves `/dashboard` (the group is stripped from the URL),
        // and the group's layer wraps it there.
        let (status, _, body) = send(&router, Method::GET, "/dashboard");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"route");
        assert_eq!(*trace.lock().unwrap(), vec!["auth"]);
    }

    #[test]
    fn routes_sharing_a_url_with_the_same_layers_build() {
        // Different group spellings are fine as long as the same layers apply.
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/(a)/x"), say_route))
                .route(RouteFn::new(Method::POST, path("/(b)/x"), say_posted))
                .layer(LayerFn::new(Some(path("/")), trace_root)),
        );
        let (status, _, body) = send(&router, Method::POST, "/x");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"posted");
        assert_eq!(*trace.lock().unwrap(), vec!["root"]);
    }

    #[test]
    fn routes_sharing_a_url_keep_their_own_layers() {
        // Both routes serve `/x`, but the layer inside `(auth)` wraps only
        // the route registered through that group.
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/(auth)/x"), say_route))
                .route(RouteFn::new(Method::POST, path("/(open)/x"), say_posted))
                .layer(LayerFn::new(Some(path("/(auth)")), trace_auth)),
        );

        let (status, _, body) = send(&router, Method::GET, "/x");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"route");
        assert_eq!(*trace.lock().unwrap(), vec!["auth"]);

        trace.lock().unwrap().clear();
        let (status, _, body) = send(&router, Method::POST, "/x");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"posted");
        assert!(trace.lock().unwrap().is_empty());
    }

    #[test]
    fn method_not_allowed_runs_only_the_pathless_layers() {
        // A 405 belongs to no route, so only the layers without a path wrap
        // it; neither the group layer nor the root-path layer runs.
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/(auth)/x"), say_route))
                .layer(LayerFn::new(Some(path("/(auth)")), trace_auth))
                .layer(LayerFn::new(Some(path("/")), trace_root))
                .layer(LayerFn::new(None::<&Path>, trace_always)),
        );
        let (status, _, _) = send(&router, Method::POST, "/x");
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(*trace.lock().unwrap(), vec!["always"]);
    }

    // -- Router::handle: trailing slash --

    /// A router with routes at a slash-less path, a slashed path, a
    /// parameter path, a catch-all, and the root, under `policy`.
    fn trailing_slash_router(policy: TrailingSlash) -> Router {
        RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/"), say_route))
            .route(RouteFn::new(Method::GET, path("/bare"), say_route))
            .route(RouteFn::new(Method::GET, path("/slashed/"), say_route))
            .route(RouteFn::new(Method::GET, path("/users/{id}"), echo_params))
            .route(RouteFn::new(
                Method::GET,
                path("/files/{*rest}"),
                echo_params,
            ))
            .route(RouteFn::new(Method::POST, path("/echo-body"), echo_body))
            .trailing_slash(policy)
            .build()
    }

    #[test]
    fn the_declared_form_is_served() {
        for policy in [
            TrailingSlash::Redirect,
            TrailingSlash::Serve,
            TrailingSlash::Strict,
        ] {
            let router = trailing_slash_router(policy);
            for requested in ["/", "/bare", "/slashed/", "/users/42", "/files/a/b"] {
                let (status, _, _) = send(&router, Method::GET, requested);
                assert_eq!(status, StatusCode::OK, "for `{requested}` under {policy:?}");
            }
        }
    }

    #[test]
    fn redirect_sends_the_other_form_to_the_declared_form() {
        let router = trailing_slash_router(TrailingSlash::Redirect);
        for (requested, declared) in [
            ("/bare/", "/bare"),
            ("/slashed", "/slashed/"),
            ("/users/42/", "/users/42"),
            ("/bare/?a=1&b=2", "/bare?a=1&b=2"),
            ("/slashed?a=1", "/slashed/?a=1"),
        ] {
            let (status, headers, _) = send(&router, Method::GET, requested);
            assert_eq!(status, StatusCode::PERMANENT_REDIRECT, "for `{requested}`");
            assert_eq!(headers["location"], declared, "for `{requested}`");
        }
    }

    #[test]
    fn redirect_applies_to_every_method() {
        // The 308 tells the client to resubmit with the same method and body,
        // so the method is not checked before redirecting.
        let router = trailing_slash_router(TrailingSlash::Redirect);
        let (status, headers, _) = send(&router, Method::POST, "/echo-body/");
        assert_eq!(status, StatusCode::PERMANENT_REDIRECT);
        assert_eq!(headers["location"], "/echo-body");
        let (status, _, _) = send(&router, Method::DELETE, "/bare/");
        assert_eq!(status, StatusCode::PERMANENT_REDIRECT);
    }

    #[test]
    fn a_catch_all_captures_the_trailing_slash_itself() {
        let router = trailing_slash_router(TrailingSlash::Redirect);
        let (status, _, body) = send(&router, Method::GET, "/files/a/");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"rest=a/");
    }

    #[test]
    fn redirect_runs_the_pathless_layers_only() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/admin/x"), say_route))
                .layer(LayerFn::new(None::<&Path>, trace_always))
                .layer(LayerFn::new(Some(path("/admin")), trace_admin)),
        );
        let (status, _, _) = send(&router, Method::GET, "/admin/x/");
        assert_eq!(status, StatusCode::PERMANENT_REDIRECT);
        assert_eq!(*trace.lock().unwrap(), vec!["always"]);
    }

    #[test]
    fn serve_handles_both_forms() {
        let router = trailing_slash_router(TrailingSlash::Serve);
        for (requested, expected) in [
            ("/bare/", "route"),
            ("/slashed", "route"),
            ("/users/42/", "id=42"),
        ] {
            let (status, _, body) = send(&router, Method::GET, requested);
            assert_eq!(status, StatusCode::OK, "for `{requested}`");
            assert_eq!(&body[..], expected.as_bytes(), "for `{requested}`");
        }
    }

    #[test]
    fn serve_keeps_the_requested_uri() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/x"), echo_uris))
            .route(RouteFn::new(Method::GET, path("/y"), echo_endpoint_path))
            .trailing_slash(TrailingSlash::Serve)
            .build();
        let (_, _, body) = send(&router, Method::GET, "/x/");
        assert_eq!(&body[..], b"/x/ /x/");
        // The endpoint is the one registered for the other form.
        let (_, _, body) = send(&router, Method::GET, "/y/");
        assert_eq!(&body[..], b"/y/");
    }

    #[test]
    fn serve_checks_the_method_on_the_other_form() {
        let router = trailing_slash_router(TrailingSlash::Serve);
        let (status, _, _) = send(&router, Method::POST, "/bare/");
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }

    #[test]
    fn strict_leaves_the_other_form_unmatched() {
        let router = trailing_slash_router(TrailingSlash::Strict);
        for requested in ["/bare/", "/slashed", "/users/42/"] {
            let (status, _, _) = send(&router, Method::GET, requested);
            assert_eq!(status, StatusCode::NOT_FOUND, "for `{requested}`");
        }
    }

    #[test]
    fn routes_at_both_forms_are_left_alone() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/x"), echo_endpoint_path))
            .route(RouteFn::new(Method::GET, path("/x/"), echo_endpoint_path))
            .build();
        let (status, _, body) = send(&router, Method::GET, "/x");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"/x");
        let (status, _, body) = send(&router, Method::GET, "/x/");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"/x/");
    }

    // -- Router::handle: pages and layouts --

    #[test]
    fn page_renders_as_html() {
        let router = RouterBuilder::new()
            .page(PageFn::new(Method::GET, path("/p"), render_page))
            .build();
        let (status, headers, body) = send(&router, Method::GET, "/p");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            headers.get(http::header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
        assert_eq!(&body[..], b"page");
    }

    #[test]
    fn rendering_panic_becomes_internal_server_error() {
        let router = RouterBuilder::new()
            .page(PageFn::new(
                Method::GET,
                path("/panic"),
                render_panicking_page,
            ))
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .build();

        let (status, _, body) = send(&router, Method::GET, "/panic");
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(&body[..], b"internal server error");

        let (status, _, body) = send(&router, Method::GET, "/x");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"route");
    }

    #[test]
    fn a_page_serves_only_its_declared_methods() {
        let router = RouterBuilder::new()
            .page(PageFn::new(Method::POST, path("/p"), render_page))
            .build();
        let (status, _, body) = send(&router, Method::POST, "/p");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"page");
        let (status, _, _) = send(&router, Method::GET, "/p");
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }

    #[test]
    fn matching_layouts_wrap_a_page_outermost_first() {
        let router = RouterBuilder::new()
            .page(PageFn::new(Method::GET, path("/admin/p"), render_page))
            .layout(LayoutFn::new(path("/admin"), layout_admin))
            .layout(LayoutFn::new(path("/"), layout_root))
            .build();

        let (status, _, body) = send(&router, Method::GET, "/admin/p");
        assert_eq!(status, StatusCode::OK);
        // Root (least specific) is outermost, admin is innermost, page deepest.
        assert_eq!(&body[..], b"R[A[page]]");
    }

    #[test]
    fn layout_only_wraps_pages_under_its_path() {
        let router = RouterBuilder::new()
            .page(PageFn::new(Method::GET, path("/p"), render_page))
            .layout(LayoutFn::new(path("/admin"), layout_admin))
            .build();
        // The `/admin` layout does not apply to a page at `/p`.
        let (_, _, body) = send(&router, Method::GET, "/p");
        assert_eq!(&body[..], b"page");
    }

    // -- Router::handle: contexts that outlive the handler --

    /// Registers the greeting the streaming route reads back.
    #[cfg(feature = "sse")]
    fn insert_greeting<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        let cx = cx.with(Greeting("hello"));
        Box::pin(async move { next.run(&cx, body).await })
    }

    /// Streams the request-context greeting from a body that outlives the
    /// handler, reading it through an owned clone of the context.
    #[cfg(feature = "sse")]
    fn stream_greeting(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        use crate::content::sse::{Event, Sse};

        Box::pin(async move {
            let handle = cx.clone();
            let events = futures_util::stream::once(async move {
                Result::<Event>::Ok(Event::new().data(request_context::<Greeting>(&handle).0))
            });
            Sse::new(events).into_response(cx)
        })
    }

    #[cfg(feature = "sse")]
    #[test]
    fn a_cloned_handle_serves_a_stream_after_the_request_returned() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/events"), stream_greeting))
            .layer(LayerFn::new(Some(path("/")), insert_greeting))
            .build();

        let response = block_on(router.handle(request(Method::GET, "/events")));
        // The router dropped its own context when `handle` returned; the body
        // still reads the request context through its owned clone.
        let body = block_on(to_bytes(response.into_body(), usize::MAX)).unwrap();
        assert!(body.starts_with(b"data: hello"), "{body:?}");
    }
}
