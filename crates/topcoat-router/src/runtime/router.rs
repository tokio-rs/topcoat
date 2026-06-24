use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use topcoat_core::runtime::context::{ContextMap, Cx};

use crate::runtime::{
    Endpoint, Layer, LayoutFn, Next, PageFn, PageWithLayouts, Path, RawPathParams, Request,
    Response, Route, Terminal, respond,
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
    routes: Vec<Box<dyn Route>>,
    /// The endpoint handling each path, matched against the request URL and
    /// indexing into `routes` by HTTP method.
    endpoints: matchit::Router<Endpoint>,
    /// The layers wrapping every matched route, in registration order; the
    /// last-registered layer is the outermost and runs first.
    layers: Vec<Box<dyn Layer>>,
    /// The values shared by every request, read back via
    /// [`app_context`](topcoat_core::runtime::context::app_context).
    app_context: Arc<ContextMap>,
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
    /// Returns `404 Not Found` when no route matches the path, or
    /// `405 Method Not Allowed` (with an `Allow` header) when the path matches
    /// but the method does not.
    pub async fn handle(&self, request: Request) -> Response {
        let (parts, body) = request.into_parts();

        // Resolve the layer stack and the chain's terminal. A matched path
        // reuses its endpoint's precomputed layer stack — whether the method
        // matches (a route) or not (405) — so both flow through the same layers.
        // An unmatched path (404) has no precomputed stack, so its layers are
        // selected from the request path on this cold path.
        let not_found_layers: Vec<usize>;
        let (layers, terminal, path_params) =
            if let Ok(matched) = self.endpoints.at(parts.uri.path()) {
                let path_params = RawPathParams::from_pairs(matched.params.iter());
                let terminal = match matched.value.get(&parts.method) {
                    Some(index) => Terminal::Route(&*self.routes[index]),
                    None => Terminal::MethodNotAllowed(matched.value),
                };
                (matched.value.layers(), terminal, path_params)
            } else {
                not_found_layers = layers_for(request_path(&parts.uri), &self.layers);
                (
                    not_found_layers.as_slice(),
                    Terminal::NotFound,
                    RawPathParams::default(),
                )
            };

        let mut cx = Cx::new(self.app_context.clone(), ContextMap::new());
        cx.insert(path_params);
        cx.insert(parts);

        let next = Next::new(&self.layers, layers, terminal);
        respond(next.run(&mut cx, body).await)
    }
}

/// Parses a request URI's path into a [`Path`], falling back to the root path
/// when it is malformed so layer selection still resolves the root layers.
fn request_path(uri: &http::Uri) -> &Path {
    Path::from_str(uri.path()).unwrap_or(Path::new("/"))
}

/// Selects the layers whose path is a prefix of `path`, as indices into
/// `layers`, ordered least- to most-specific so the outermost layer runs first.
///
/// Reversing the filtered indices before the stable sort means that among
/// layers sharing a path the most recently registered ends up outermost.
fn layers_for(path: &Path, layers: &[Box<dyn Layer>]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..layers.len())
        .filter(|&i| path.starts_with(&layers[i].path()))
        .rev()
        .collect();
    indices.sort_by_key(|&i| layers[i].path().len());
    indices
}

/// Builds a [`Router`] for a Topcoat application.
///
/// This is the common construction surface used by manual routing,
/// auto-discovery, `module_router!`, and builder extension traits. Register
/// [`page`](Self::page), [`layout`](Self::layout), [`layer`](Self::layer), and
/// [`route`](Self::route) handlers directly, or let a discovery helper add
/// them, then call [`build`](Self::build) once at the end.
///
/// Builder extension traits add application-wide behavior before finalization,
/// such as assets, cookies, or typed [`app_context`](Self::app_context) values.
///
/// # Examples
///
/// ```rust
/// # struct AppConfig;
/// # impl AppConfig { fn load() -> Self { Self } }
/// use topcoat::{
///     asset::{AssetBundle, RouterBuilderAssetExt},
///     cookie::RouterBuilderCookieExt,
///     router::{Router, RouterBuilderDiscoverExt},
/// };
///
/// pub fn router() -> Router {
///     Router::builder()
///         .discover()
///         .cookies()
///         .assets(AssetBundle::load().unwrap())
///         .app_context(AppConfig::load())
///         .build()
/// }
/// ```
pub struct RouterBuilder {
    routes: Vec<Box<dyn Route>>,
    pages: Vec<PageFn>,
    layouts: Vec<LayoutFn>,
    layers: Vec<Box<dyn Layer>>,
    context: ContextMap,
}

impl RouterBuilder {
    /// Creates an empty builder with no routes registered.
    #[must_use]
    pub fn new() -> Self {
        let mut context = ContextMap::new();
        // Register `()` so APIs generic over an app context type can default to `S = ()`.
        context.insert(());
        Self {
            routes: Vec::new(),
            pages: Vec::new(),
            layouts: Vec::new(),
            layers: Vec::new(),
            context,
        }
    }

    /// Returns `true` if no routes, pages, or layouts have been registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty() && self.pages.is_empty() && self.layouts.is_empty()
    }

    /// Registers a [`Route`], an HTTP handler bound to a specific method and
    /// path.
    #[must_use]
    pub fn route(mut self, route: impl Route) -> Self {
        self.routes.push(Box::new(route));
        self
    }

    /// Registers every route annotated with `#[route]` and collected at link
    /// time.
    #[cfg(feature = "discover")]
    #[must_use]
    pub fn discover_routes(mut self) -> Self {
        for route in inventory::iter::<crate::runtime::RouteFn>().cloned() {
            self = self.route(route);
        }
        self
    }

    /// Registers a [`PageFn`]. Order doesn't matter — layout matching is based
    /// on path prefixes, not registration order.
    #[must_use]
    pub fn page(mut self, page: PageFn) -> Self {
        self.pages.push(page);
        self
    }

    /// Registers every [`PageFn`] annotated with `#[page]` and collected at
    /// link time.
    #[cfg(feature = "discover")]
    #[must_use]
    pub fn discover_pages(mut self) -> Self {
        for page in inventory::iter::<PageFn>().cloned() {
            self = self.page(page);
        }
        self
    }

    /// Registers a [`LayoutFn`]. A layout applies to every page whose path
    /// starts with the layout's path prefix.
    #[must_use]
    pub fn layout(mut self, layout: LayoutFn) -> Self {
        self.layouts.push(layout);
        self
    }

    /// Registers every [`LayoutFn`] annotated with `#[layout]` and collected at
    /// link time.
    ///
    /// At most one discovered layout is allowed per path: a page's layouts nest
    /// by path prefix, so two layouts sharing a path would have an undefined
    /// nesting order. To attach more than one layout to a page, give them
    /// distinct paths or compose them in a single layout component.
    ///
    /// # Panics
    ///
    /// Panics if two discovered layouts share the same path.
    #[cfg(feature = "discover")]
    #[must_use]
    pub fn discover_layouts(mut self) -> Self {
        let mut seen = std::collections::HashSet::<Cow<'static, crate::runtime::Path>>::new();
        for layout in inventory::iter::<LayoutFn>().cloned() {
            assert!(
                seen.insert(layout.path()),
                "multiple discovered layouts registered for the same path \"{}\"",
                layout.path()
            );
            self = self.layout(layout);
        }
        self
    }

    /// Registers a [`Layer`] that wraps every matched route whose path begins
    /// with the layer's path, like a layout.
    ///
    /// When layers at *different* paths match a route they nest from
    /// least-specific (outermost) to most-specific (innermost). Multiple layers
    /// may share the same path; among those, the most recently registered runs
    /// first (outermost), so `.layer(a).layer(b)` runs `b` around `a` when both
    /// sit at the same path.
    #[must_use]
    pub fn layer(mut self, layer: impl Layer) -> Self {
        self.layers.push(Box::new(layer));
        self
    }

    /// Registers every layer annotated with `#[layer]` and collected at link
    /// time.
    ///
    /// Unlike [`layer`](Self::layer), at most one discovered layer is allowed
    /// per path. Link-time collection order is non-deterministic, so two
    /// discovered layers sharing a path would have an undefined run order; this
    /// rejects that rather than pick an arbitrary one. To stack several layers
    /// on one path, register them explicitly with [`layer`](Self::layer), whose
    /// order is well-defined.
    ///
    /// # Panics
    ///
    /// Panics if two discovered layers share the same path.
    #[cfg(feature = "discover")]
    #[must_use]
    pub fn discover_layers(mut self) -> Self {
        let mut seen = std::collections::HashSet::<Cow<'static, crate::runtime::Path>>::new();
        for layer in inventory::iter::<crate::runtime::LayerFn>().cloned() {
            assert!(
                seen.insert(layer.path()),
                "multiple discovered layers registered for the same path \"{}\"",
                layer.path()
            );
            self = self.layer(layer);
        }
        self
    }

    /// Registers a unique value that is accessible to every request sent to
    /// this router by its type `T`. The top-level
    /// [`app_context`](topcoat_core::runtime::context::app_context) function can be used to
    /// retrieve a reference to this value via a request context.
    ///
    /// # Panics
    ///
    /// Panics if a value has already been registered for the same type.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use topcoat::{Result, router::route};
    /// # struct User;
    /// # #[route(GET "/users")]
    /// # async fn get_user() -> Result<&'static str> { Ok("ok") }
    /// use topcoat::context::{Cx, app_context};
    /// use topcoat::router::Router;
    ///
    /// struct Database {/* ... */}
    /// # impl Database {
    /// #     fn connect() -> Self { Self {} }
    /// #     async fn fetch_user(&self, _id: u64) -> User { User }
    /// # }
    ///
    /// pub fn router() -> Router {
    ///     Router::builder()
    ///         .route(get_user)
    ///         .app_context(Database::connect())
    ///         .build()
    /// }
    ///
    /// async fn fetch_user(cx: &Cx, id: u64) -> User {
    ///     let db: &Database = app_context(cx);
    ///     db.fetch_user(id).await
    /// }
    /// ```
    #[must_use]
    pub fn app_context<T>(mut self, value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        self.context.insert(value);
        self
    }

    /// Finalizes the registered routes, pages, and layouts into a [`Router`].
    ///
    /// # Panics
    ///
    /// Panics if two routes resolve to the same path and HTTP method, since the
    /// router would have no way to choose between them.
    #[must_use]
    pub fn build(self) -> Router {
        let RouterBuilder {
            mut routes,
            pages,
            layouts,
            layers,
            context,
        } = self;

        // Wire each page to the layouts whose path is a prefix of the page's,
        // ordered from least- to most-specific so the page nests innermost.
        for page in pages {
            let mut matching: Vec<LayoutFn> = layouts
                .iter()
                .filter(|layout| page.path().starts_with(&layout.path()))
                .cloned()
                .collect();
            matching.sort_by_key(|layout| layout.path().len());
            routes.push(Box::new(PageWithLayouts::new(page, matching)));
        }

        // Group routes that share a path into a single endpoint first, since
        // matchit rejects inserting the same path twice. Two routes that resolve
        // to the same path *and* method are ambiguous, so reject them here.
        // Remember each group's first route path (with its group segments
        // intact, which `to_matchit_path` strips) to select the endpoint's
        // layers below.
        let mut grouped: HashMap<Cow<'static, str>, Endpoint> = HashMap::new();
        let mut paths: HashMap<Cow<'static, str>, Cow<'static, Path>> = HashMap::new();
        for (index, route) in routes.iter().enumerate() {
            let route_path = route.path();
            let path = route_path.to_matchit_path();
            let method = route.method();
            let endpoint = grouped.entry(path.clone()).or_default();
            assert!(
                endpoint.get(&method).is_none(),
                "duplicate route registered for `{method} {path}`"
            );
            endpoint.insert(method, index);
            paths.entry(path).or_insert(route_path);
        }

        // Precompute the layer stack wrapping each endpoint once, so dispatch
        // only has to index into it rather than filter and sort per request.
        let mut endpoints = matchit::Router::new();
        for (path, mut endpoint) in grouped {
            endpoint.alias_head_to_get();
            endpoint.set_layers(layers_for(&paths[&path], &layers).into_boxed_slice());
            endpoints
                .insert(path.clone(), endpoint)
                .unwrap_or_else(|error| panic!("failed to register route {path:?}: {error}"));
        }

        Router {
            routes,
            endpoints,
            layers,
            app_context: Arc::new(context),
        }
    }
}

impl Default for RouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Mutex;

    use http::{HeaderMap, StatusCode};
    use topcoat_core::runtime::context::{app_context, request_context};
    use topcoat_core::runtime::error::Result;
    use topcoat_view::runtime::{View, ViewParts};

    use super::*;
    use crate::runtime::{
        Body, Bytes, IntoResponse, LayerFn, LayerFuture, Method, RouteFn, RouteFuture, Slot,
        to_bytes,
    };

    // ── Test helpers ──

    fn block_on<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap()
            .block_on(future)
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

    /// A `Cow<'static, Path>` from a path literal, for constructing routes.
    fn path(s: &'static str) -> Cow<'static, Path> {
        Cow::Borrowed(Path::new(s))
    }

    // A handful of plain handler functions, since `Route`/`Layer` are backed by
    // `fn` pointers and cannot capture state.

    fn say_route(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { "route".into_response() })
    }

    fn say_posted(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { "posted".into_response() })
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
                .into_response()
        })
    }

    /// Reads a registered app-context greeting and returns it as the body.
    struct Greeting(&'static str);

    fn say_greeting(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { app_context::<Greeting>(cx).0.into_response() })
    }

    // Layers that record their label in a shared trace before continuing, so a
    // test can observe the order layers run in.
    type Trace = Mutex<Vec<&'static str>>;

    fn trace_root<'a>(cx: &'a mut Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            app_context::<Arc<Trace>>(cx).lock().unwrap().push("root");
            next.run(cx, body).await
        })
    }

    fn trace_admin<'a>(cx: &'a mut Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move {
            app_context::<Arc<Trace>>(cx).lock().unwrap().push("admin");
            next.run(cx, body).await
        })
    }

    // Page and layout render functions for the rendering tests.
    type ViewFuture<'cx> = Pin<Box<dyn Future<Output = Result<View>> + Send + 'cx>>;

    fn view(text: &'static str) -> View {
        let mut parts = ViewParts::new();
        parts.push(text);
        View::new(parts)
    }

    fn render_page(_cx: &Cx, _body: Body) -> ViewFuture<'_> {
        Box::pin(async move { Ok(view("page")) })
    }

    /// Wraps the child content in `R[ … ]` so layout nesting is observable.
    fn layout_root<'cx>(_cx: &'cx Cx, slot: Slot<'cx>) -> ViewFuture<'cx> {
        Box::pin(async move {
            let inner = slot.await?;
            let mut parts = ViewParts::new();
            parts.push("R[");
            parts.push(inner);
            parts.push("]");
            Ok(View::new(parts))
        })
    }

    /// Wraps the child content in `A[ … ]`.
    fn layout_admin<'cx>(_cx: &'cx Cx, slot: Slot<'cx>) -> ViewFuture<'cx> {
        Box::pin(async move {
            let inner = slot.await?;
            let mut parts = ViewParts::new();
            parts.push("A[");
            parts.push(inner);
            parts.push("]");
            Ok(View::new(parts))
        })
    }

    // ── request_path ──

    #[test]
    fn request_path_reads_a_valid_path() {
        let uri: http::Uri = "/users/42".parse().unwrap();
        assert_eq!(request_path(&uri), Path::new("/users/42"));
    }

    #[test]
    fn request_path_falls_back_to_root_when_malformed() {
        // A doubled slash leaves an empty segment, which is not a valid `Path`.
        let uri: http::Uri = "/a//b".parse().unwrap();
        assert_eq!(request_path(&uri), Path::new("/"));
    }

    // ── layers_for ──

    /// A layer that only carries a path; its `handle` is never invoked by
    /// `layers_for`, which inspects paths alone.
    struct PathLayer(Cow<'static, Path>);

    impl Layer for PathLayer {
        fn path(&self) -> Cow<'static, Path> {
            self.0.clone()
        }

        fn handle<'a>(&'a self, _cx: &'a mut Cx, _body: Body, _next: Next<'a>) -> LayerFuture<'a> {
            unreachable!("layers_for never runs a layer")
        }
    }

    fn path_layers(paths: &[&'static str]) -> Vec<Box<dyn Layer>> {
        paths
            .iter()
            .map(|&p| Box::new(PathLayer(path(p))) as Box<dyn Layer>)
            .collect()
    }

    #[test]
    fn layers_for_selects_prefixes_least_specific_first() {
        let layers = path_layers(&["/admin", "/", "/admin/users", "/other"]);
        // For `/admin/users/1` the matching layers are `/`, `/admin`,
        // `/admin/users`, ordered by ascending path length (outermost first).
        let indices = layers_for(Path::new("/admin/users/1"), &layers);
        assert_eq!(indices, vec![1, 0, 2]);
    }

    #[test]
    fn layers_for_excludes_non_prefixes() {
        let layers = path_layers(&["/admin", "/blog"]);
        assert_eq!(layers_for(Path::new("/admin/x"), &layers), vec![0]);
        assert_eq!(
            layers_for(Path::new("/marketing"), &layers),
            Vec::<usize>::new()
        );
    }

    #[test]
    fn layers_for_orders_same_path_most_recent_first() {
        // Two layers share the root path; the later-registered one (index 1)
        // ends up outermost.
        let layers = path_layers(&["/", "/"]);
        assert_eq!(layers_for(Path::new("/anything"), &layers), vec![1, 0]);
    }

    // ── RouterBuilder ──

    #[test]
    fn new_builder_is_empty() {
        let builder = RouterBuilder::new();
        assert!(builder.is_empty());
    }

    #[test]
    fn builder_is_not_empty_after_registering_a_route() {
        let builder = RouterBuilder::new().route(RouteFn::new(Method::GET, path("/x"), say_route));
        assert!(!builder.is_empty());
    }

    #[test]
    #[should_panic(expected = "duplicate route")]
    fn duplicate_method_and_path_panics_on_build() {
        let _ = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .route(RouteFn::new(Method::GET, path("/x"), say_route))
            .build();
    }

    #[test]
    #[should_panic(expected = "duplicate context entry")]
    fn duplicate_app_context_type_panics() {
        let _ = RouterBuilder::new()
            .app_context(Greeting("a"))
            .app_context(Greeting("b"));
    }

    // ── Router::handle: dispatch ──

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
    fn app_context_is_available_to_handlers() {
        let router = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/hi"), say_greeting))
            .app_context(Greeting("hello"))
            .build();
        let (status, _, body) = send(&router, Method::GET, "/hi");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(&body[..], b"hello");
    }

    // ── Router::handle: layers ──

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
    fn layers_wrap_not_found_responses() {
        let (router, trace) =
            trace_router(RouterBuilder::new().layer(LayerFn::new(path("/"), trace_root)));
        let (status, _, _) = send(&router, Method::GET, "/missing");
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(*trace.lock().unwrap(), vec!["root"]);
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

    // ── Router::handle: pages and layouts ──

    #[test]
    fn page_renders_as_html() {
        let router = RouterBuilder::new()
            .page(PageFn::new(path("/p"), render_page))
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
    fn matching_layouts_wrap_a_page_outermost_first() {
        let router = RouterBuilder::new()
            .page(PageFn::new(path("/admin/p"), render_page))
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
            .page(PageFn::new(path("/p"), render_page))
            .layout(LayoutFn::new(path("/admin"), layout_admin))
            .build();
        // The `/admin` layout does not apply to a page at `/p`.
        let (_, _, body) = send(&router, Method::GET, "/p");
        assert_eq!(&body[..], b"page");
    }
}
