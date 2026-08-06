use std::{
    future::{Future, poll_fn},
    panic::{AssertUnwindSafe, catch_unwind},
    pin::pin,
    sync::Arc,
    task::Poll,
};

use topcoat_core::context::{ContextMap, Cx, CxBuilder};

use crate::{
    Endpoint, Layer, Layers, Next, OriginLayer, RawPathParams, Route, RouterBuilder, Terminal,
    error::{internal_server_response, not_found, respond},
    request::Request,
    response::Response,
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
    /// The registered routes, indexed by the values stored in `endpoints`.
    pub(crate) routes: Vec<Box<dyn Route>>,
    /// The endpoint handling each path, matched against the request URL and
    /// indexing into `routes` by HTTP method.
    pub(crate) endpoints: matchit::Router<Endpoint>,
    /// The layers registered on this router, wrapping matched routes by path
    /// prefix.
    pub(crate) layers: Layers,
    /// The values shared by every request, read back via
    /// [`app_context`](topcoat_core::context::app_context).
    pub(crate) app_context: Arc<ContextMap>,
    /// The origin policy wrapping every request as the outermost layer.
    pub(crate) origin: OriginLayer,
    /// The compression applied to responses on their way out.
    #[cfg(feature = "compression")]
    pub(crate) compression: crate::Compression,
}

impl Router {
    /// Creates an empty [`RouterBuilder`].
    #[must_use]
    pub fn builder() -> RouterBuilder {
        RouterBuilder::new()
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
        let (parts, body) = request.into_parts();

        let Ok(matched) = self.endpoints.at(parts.uri.path()) else {
            return topcoat_view::scope(async { respond(&Cx::default(), not_found()) }).await;
        };

        // The chain's terminal, reached through the endpoint's precomputed
        // layer stack whether the method matches (a route) or not (405), so
        // both flow through the same layers.
        let endpoint = matched.value;
        let path_params = {
            debug_assert_eq!(endpoint.path_params().len(), matched.params.len());
            let keys = endpoint.path_params().iter().cloned();
            let values = matched.params.iter().map(|(_, value)| value);
            RawPathParams::from_pairs(keys.zip(values))
        };
        let terminal = match endpoint.get(&parts.method).or_else(|| endpoint.any()) {
            Some(index) => Terminal::Route(&*self.routes[index]),
            None => Terminal::MethodNotAllowed(endpoint),
        };

        let mut cx = CxBuilder::new(self.app_context.clone());
        cx.insert(path_params);
        cx.insert(parts);

        // The whole chain runs inside one view scope, so every view built
        // while handling the request shares the scope's instruction memory
        // and rendering the response can execute it.
        let response = topcoat_view::scope(async {
            // The origin layer wraps the whole chain, denying untrusted
            // cross-origin requests before anything else runs.
            let next = Next::new(&self.layers, endpoint.layers(), terminal);
            let response = self.origin.handle(&mut cx, body, next).await;
            respond(&cx, response)
        })
        .await;

        // Compression runs outside every layer, so layers see uncompressed
        // bodies. The negotiation reads the request headers as the layers
        // left them.
        #[cfg(feature = "compression")]
        let response = match cx.get::<http::request::Parts>() {
            Some(parts) => self.compression.compress(&parts.headers, response).await,
            None => response,
        };

        response
    }
}

#[cfg(test)]
mod tests {
    use std::{
        borrow::Cow,
        future::Future,
        pin::Pin,
        sync::{Arc, Mutex},
    };

    use http::{HeaderMap, StatusCode};
    use topcoat_core::{
        context::{Cx, CxBuilder, app_context, request_context},
        error::Result,
    };
    use topcoat_view::{DynViewPart, HtmlWriter, View, internal::__build_view};

    use super::*;
    use crate::{
        Body, LayerFn, LayerFuture, LayoutFn, Method, Methods, OriginPolicy, PageFn, Path, RouteFn,
        RouteFuture, request::Bytes, response::IntoResponse, to_bytes,
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
            let params: &RawPathParams = request_context(cx);
            params
                .into_iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join("&")
                .into_response(cx)
        })
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

    fn trace_root<'a>(cx: &'a mut CxBuilder, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            app_context::<Arc<Trace>>(cx).lock().unwrap().push("root");
            next.run(cx, body).await
        })
    }

    fn trace_admin<'a>(cx: &'a mut CxBuilder, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            app_context::<Arc<Trace>>(cx).lock().unwrap().push("admin");
            next.run(cx, body).await
        })
    }

    fn trace_auth<'a>(cx: &'a mut CxBuilder, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            app_context::<Arc<Trace>>(cx).lock().unwrap().push("auth");
            next.run(cx, body).await
        })
    }

    // Page and layout render functions for the rendering tests.
    type ViewFuture<'cx> = Pin<Box<dyn Future<Output = Result<View>> + Send + 'cx>>;

    fn view(text: &'static str) -> View {
        __build_view(|parts| {
            parts.push_str(text);
        })
    }

    fn render_page(_cx: &Cx, _body: Body) -> ViewFuture<'_> {
        Box::pin(async move { Ok(view("page")) })
    }

    #[derive(Debug, Clone)]
    struct PanickingViewPart;

    impl DynViewPart for PanickingViewPart {
        fn render(&self, _cx: &Cx, _w: &mut HtmlWriter<'_, '_>) {
            panic!("view rendering panicked");
        }
    }

    fn render_panicking_page(_cx: &Cx, _body: Body) -> ViewFuture<'_> {
        Box::pin(async move {
            Ok(__build_view(|parts| {
                parts.push_dyn(Box::new(PanickingViewPart));
            }))
        })
    }

    /// Wraps the child content in `R[ ... ]` so layout nesting is observable.
    fn layout_root(_cx: &Cx, slot: Result<View>) -> ViewFuture<'_> {
        Box::pin(async move {
            let inner = slot?;
            Ok(__build_view(|parts| {
                parts.push_str("R[");
                parts.push_view(inner);
                parts.push_str("]");
            }))
        })
    }

    /// Wraps the child content in `A[ ... ]`.
    fn layout_admin(_cx: &Cx, slot: Result<View>) -> ViewFuture<'_> {
        Box::pin(async move {
            let inner = slot?;
            Ok(__build_view(|parts| {
                parts.push_str("A[");
                parts.push_view(inner);
                parts.push_str("]");
            }))
        })
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
                .layer(LayerFn::new(path("/admin"), trace_admin))
                .layer(LayerFn::new(path("/"), trace_root)),
        );

        let (status, _, body) = send(&router, Method::GET, "/admin/x");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"route");
        // The root layer (least specific) wraps the admin layer.
        assert_eq!(*trace.lock().unwrap(), vec!["root", "admin"]);
    }

    #[test]
    fn layers_only_wrap_routes_under_their_path() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/admin/x"), say_route))
                .route(RouteFn::new(Method::GET, path("/public"), say_route))
                .layer(LayerFn::new(path("/admin"), trace_admin)),
        );

        send(&router, Method::GET, "/public");
        assert!(trace.lock().unwrap().is_empty());

        send(&router, Method::GET, "/admin/x");
        assert_eq!(*trace.lock().unwrap(), vec!["admin"]);
    }

    #[test]
    fn layers_do_not_wrap_not_found_responses() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/admin/x"), say_route))
                .layer(LayerFn::new(path("/"), trace_root)),
        );

        let (status, _, _) = send(&router, Method::GET, "/missing");
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(trace.lock().unwrap().is_empty());

        // A trailing slash is a different URL: the route does not match, and
        // the unmatched path is answered without running any layers.
        let (status, _, _) = send(&router, Method::GET, "/admin/x/");
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert!(trace.lock().unwrap().is_empty());
    }

    #[test]
    fn layers_wrap_method_not_allowed_responses() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/x"), say_route))
                .layer(LayerFn::new(path("/"), trace_root)),
        );
        let (status, _, _) = send(&router, Method::POST, "/x");
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(*trace.lock().unwrap(), vec!["root"]);
    }

    #[test]
    fn dispatch_ignores_query_strings() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/admin/x"), say_route))
                .layer(LayerFn::new(path("/admin"), trace_admin)),
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
                .layer(LayerFn::new(path("/admin"), trace_admin)),
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
                .layer(LayerFn::new(path("/admin"), trace_admin)),
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
                .layer(LayerFn::new(path("/(auth)"), trace_auth)),
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
                .layer(LayerFn::new(path("/"), trace_root)),
        );
        let (status, _, body) = send(&router, Method::POST, "/x");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"posted");
        assert_eq!(*trace.lock().unwrap(), vec!["root"]);
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
}
