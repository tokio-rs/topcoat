use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use http::{HeaderValue, Method, StatusCode};
use topcoat_core::runtime::context::{ContextMap, Cx};

use crate::runtime::{
    Body, Endpoint, Layer, LayoutFn, Next, PageFn, PageWithLayouts, RawPathParams, Request,
    Response, Route, not_found, respond,
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
/// ```rust,ignore
/// use topcoat::router::{Router, RouterBuilderDiscoverExt};
///
/// let router = Router::builder()
///     .discover()
///     .build();
///
/// topcoat::start(router).await?;
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

        let Ok(matched) = self.endpoints.at(parts.uri.path()) else {
            return respond(not_found());
        };
        let Some(index) = matched.value.get(&parts.method) else {
            return method_not_allowed(matched.value);
        };
        let path_params = RawPathParams::from_pairs(matched.params.iter());

        // Select the layers whose path is a prefix of the route's, ordered
        // least- to most-specific so the outermost layer runs first. Reversing
        // before the (stable) sort means that among layers sharing a path the
        // most recently registered ends up outermost.
        let route = &*self.routes[index];
        let route_path = route.path();
        let mut layers: Vec<&dyn Layer> = self
            .layers
            .iter()
            .map(|layer| &**layer)
            .filter(|layer| route_path.starts_with(&layer.path()))
            .rev()
            .collect();
        layers.sort_by_key(|layer| layer.path().len());

        let mut cx = Cx::new(self.app_context.clone(), ContextMap::new());
        cx.insert(path_params);
        cx.insert(parts);

        let next = Next::new(&layers, route);
        respond(next.run(&mut cx, body).await)
    }
}

/// Builds a `405 Method Not Allowed` response whose `Allow` header lists the
/// methods the matched endpoint actually supports.
fn method_not_allowed(endpoint: &Endpoint) -> Response {
    let allow = endpoint
        .methods()
        .map(Method::as_str)
        .collect::<Vec<_>>()
        .join(", ");

    let mut response = Response::new(Body::empty());
    *response.status_mut() = StatusCode::METHOD_NOT_ALLOWED;
    if let Ok(allow) = HeaderValue::from_str(&allow) {
        response.headers_mut().insert(http::header::ALLOW, allow);
    }
    response
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
/// ```rust,ignore
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
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty() && self.pages.is_empty() && self.layouts.is_empty()
    }

    /// Registers a [`Route`], an HTTP handler bound to a specific method and
    /// path.
    pub fn route(mut self, route: impl Route) -> Self {
        self.routes.push(Box::new(route));
        self
    }

    /// Registers every route annotated with `#[route]` and collected at link
    /// time.
    #[cfg(feature = "discover")]
    pub fn discover_routes(mut self) -> Self {
        for route in inventory::iter::<crate::runtime::RouteFn>().cloned() {
            self = self.route(route);
        }
        self
    }

    /// Registers a [`PageFn`]. Order doesn't matter — layout matching is based
    /// on path prefixes, not registration order.
    pub fn page(mut self, page: PageFn) -> Self {
        self.pages.push(page);
        self
    }

    /// Registers every [`PageFn`] annotated with `#[page]` and collected at
    /// link time.
    #[cfg(feature = "discover")]
    pub fn discover_pages(mut self) -> Self {
        for page in inventory::iter::<PageFn>().cloned() {
            self = self.page(page);
        }
        self
    }

    /// Registers a [`LayoutFn`]. A layout applies to every page whose path
    /// starts with the layout's path prefix.
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
    pub fn discover_layouts(mut self) -> Self {
        let mut seen = std::collections::HashSet::<Cow<'static, crate::runtime::Path>>::new();
        for layout in inventory::iter::<LayoutFn>().cloned() {
            if !seen.insert(layout.path()) {
                panic!(
                    "multiple discovered layouts registered for the same path \"{}\"",
                    layout.path()
                );
            }
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
    pub fn discover_layers(mut self) -> Self {
        let mut seen = std::collections::HashSet::<Cow<'static, crate::runtime::Path>>::new();
        for layer in inventory::iter::<crate::runtime::LayerFn>().cloned() {
            if !seen.insert(layer.path()) {
                panic!(
                    "multiple discovered layers registered for the same path \"{}\"",
                    layer.path()
                );
            }
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
    /// ```rust,ignore
    /// use topcoat::context::{Cx, app_context};
    /// use topcoat::router::Router;
    ///
    /// struct Database { /* ... */ }
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
    ///     db.fetch_user(id).await;
    /// }
    /// ```
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
        let mut grouped: HashMap<Cow<'static, str>, Endpoint> = HashMap::new();
        for (index, route) in routes.iter().enumerate() {
            let path = route.path().to_matchit_path();
            let method = route.method();
            let endpoint = grouped.entry(path.clone()).or_default();
            if endpoint.get(&method).is_some() {
                panic!("duplicate route registered for `{method} {path}`");
            }
            endpoint.insert(method, index);
        }

        let mut endpoints = matchit::Router::new();
        for (path, mut endpoint) in grouped {
            endpoint.alias_head_to_get();
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
    use std::pin::Pin;

    use bytes::Bytes;
    use http::header::{HeaderName, HeaderValue};
    use topcoat_core::runtime::error::Result;
    use topcoat_view::runtime::{View, ViewParts};

    use super::*;
    use crate::runtime::{
        IntoResponse, LayerFuture, LayoutFn, PageFn, Path, RouteFuture, Slot, to_bytes,
    };

    const TRACE: HeaderName = HeaderName::from_static("x-trace");

    /// A `Route` that replies with a fixed body, used to observe which route a
    /// request was dispatched to.
    struct TestRoute {
        method: Method,
        path: &'static str,
        body: &'static str,
    }

    impl TestRoute {
        fn new(method: Method, path: &'static str, body: &'static str) -> Self {
            Self { method, path, body }
        }
    }

    impl Route for TestRoute {
        fn method(&self) -> Method {
            self.method.clone()
        }

        fn path(&self) -> Cow<'static, Path> {
            Cow::Borrowed(Path::new(self.path))
        }

        fn handle<'cx>(&'cx self, _cx: &'cx Cx, _body: Body) -> RouteFuture<'cx> {
            let body = self.body;
            Box::pin(async move { body.into_response() })
        }
    }

    /// A `Layer` that appends its label to the `x-trace` response header, so a
    /// test can observe which layers ran and in what order. Because each layer
    /// appends *after* `next` returns, the innermost layer's label appears first.
    struct TraceLayer {
        path: &'static str,
        label: &'static str,
    }

    impl TraceLayer {
        fn new(path: &'static str, label: &'static str) -> Self {
            Self { path, label }
        }
    }

    impl Layer for TraceLayer {
        fn path(&self) -> Cow<'static, Path> {
            Cow::Borrowed(Path::new(self.path))
        }

        fn handle<'a>(&'a self, cx: &'a mut Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
            let label = self.label;
            Box::pin(async move {
                let mut response = next.run(cx, body).await?;
                let trace = match response.headers().get(&TRACE).and_then(|v| v.to_str().ok()) {
                    Some(existing) => format!("{existing},{label}"),
                    None => label.to_owned(),
                };
                response
                    .headers_mut()
                    .insert(TRACE, HeaderValue::from_str(&trace).unwrap());
                Ok(response)
            })
        }
    }

    /// A page render function producing the static text "PAGE".
    fn page_render<'cx>(
        _cx: &'cx Cx,
        _body: Body,
    ) -> Pin<Box<dyn Future<Output = Result<View>> + Send + 'cx>> {
        Box::pin(async {
            let mut parts = ViewParts::new();
            parts.push("PAGE");
            Ok(View::new(parts))
        })
    }

    /// Builds a layout render function that wraps its slot in `name(...)`.
    macro_rules! wrapping_layout {
        ($name:literal) => {
            |_cx: &Cx, slot: Slot<'_>| {
                Box::pin(async move {
                    let inner = slot.await?;
                    let mut parts = ViewParts::new();
                    parts.push(concat!($name, "("));
                    parts.push(inner);
                    parts.push(")");
                    Ok(View::new(parts))
                }) as Pin<Box<dyn Future<Output = Result<View>> + Send>>
            }
        };
    }

    fn path(s: &'static str) -> Cow<'static, Path> {
        Cow::Borrowed(Path::new(s))
    }

    /// Drives the router with a request and collects the response, reading the
    /// body fully into memory.
    fn call(router: &Router, method: Method, uri: &str) -> Response<Bytes> {
        let request = Request::builder()
            .method(method)
            .uri(uri)
            .body(Body::empty())
            .unwrap();
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        runtime.block_on(async {
            let (parts, body) = router.handle(request).await.into_parts();
            let bytes = to_bytes(body, usize::MAX).await.unwrap();
            Response::from_parts(parts, bytes)
        })
    }

    fn body_str(response: &Response<Bytes>) -> &str {
        std::str::from_utf8(response.body()).unwrap()
    }

    fn trace(response: &Response<Bytes>) -> &str {
        response.headers().get(&TRACE).unwrap().to_str().unwrap()
    }

    // ── RouterBuilder ──

    #[test]
    fn builder_starts_empty() {
        assert!(RouterBuilder::new().is_empty());
    }

    #[test]
    fn builder_is_not_empty_after_registering_a_route() {
        let builder = RouterBuilder::new().route(TestRoute::new(Method::GET, "/", "home"));
        assert!(!builder.is_empty());
    }

    #[test]
    #[should_panic(expected = "duplicate context entry")]
    fn app_context_rejects_duplicate_type() {
        RouterBuilder::new().app_context(1u32).app_context(2u32);
    }

    // ── dispatch ──

    #[test]
    fn unmatched_path_is_not_found() {
        let router = RouterBuilder::new()
            .route(TestRoute::new(Method::GET, "/users", "users"))
            .build();
        let response = call(&router, Method::GET, "/missing");
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn dispatches_by_method() {
        let router = RouterBuilder::new()
            .route(TestRoute::new(Method::GET, "/users", "list"))
            .route(TestRoute::new(Method::POST, "/users", "create"))
            .build();
        assert_eq!(body_str(&call(&router, Method::GET, "/users")), "list");
        assert_eq!(body_str(&call(&router, Method::POST, "/users")), "create");
    }

    #[test]
    fn matched_path_wrong_method_is_405_with_allow_header() {
        let router = RouterBuilder::new()
            .route(TestRoute::new(Method::GET, "/users", "list"))
            .build();
        let response = call(&router, Method::DELETE, "/users");
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

        let allow = response
            .headers()
            .get(http::header::ALLOW)
            .unwrap()
            .to_str()
            .unwrap();
        // The GET route also answers HEAD after the build-time alias.
        assert!(allow.contains("GET"), "allow header was {allow:?}");
        assert!(allow.contains("HEAD"), "allow header was {allow:?}");
    }

    #[test]
    fn head_reuses_the_get_handler() {
        let router = RouterBuilder::new()
            .route(TestRoute::new(Method::GET, "/users", "list"))
            .build();
        let response = call(&router, Method::HEAD, "/users");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_str(&response), "list");
    }

    #[test]
    fn explicit_head_route_overrides_the_get_alias() {
        let router = RouterBuilder::new()
            .route(TestRoute::new(Method::GET, "/users", "list"))
            .route(TestRoute::new(Method::HEAD, "/users", "head"))
            .build();
        assert_eq!(body_str(&call(&router, Method::HEAD, "/users")), "head");
    }

    // ── layers ──

    #[test]
    fn layer_wraps_only_matching_paths() {
        let router = RouterBuilder::new()
            .route(TestRoute::new(Method::GET, "/admin/users", "admin"))
            .route(TestRoute::new(Method::GET, "/public", "public"))
            .layer(TraceLayer::new("/admin", "admin"))
            .build();

        // The route under /admin is wrapped by the /admin layer.
        assert_eq!(trace(&call(&router, Method::GET, "/admin/users")), "admin");
        // The route outside /admin is not.
        let public = call(&router, Method::GET, "/public");
        assert!(public.headers().get(&TRACE).is_none());
    }

    #[test]
    fn layers_at_different_paths_nest_outermost_first() {
        let router = RouterBuilder::new()
            .route(TestRoute::new(Method::GET, "/admin/users", "admin"))
            .layer(TraceLayer::new("/", "root"))
            .layer(TraceLayer::new("/admin", "admin"))
            .build();

        // The least-specific layer (root) is outermost, so it appends last,
        // leaving the more-specific "admin" first in the trace.
        assert_eq!(
            trace(&call(&router, Method::GET, "/admin/users")),
            "admin,root"
        );
    }

    #[test]
    fn layers_sharing_a_path_run_most_recently_registered_first() {
        let router = RouterBuilder::new()
            .route(TestRoute::new(Method::GET, "/", "home"))
            .layer(TraceLayer::new("/", "a"))
            .layer(TraceLayer::new("/", "b"))
            .build();

        // `b` is registered last, so it is outermost and appends last.
        assert_eq!(trace(&call(&router, Method::GET, "/")), "a,b");
    }

    // ── pages and layouts ──

    #[test]
    fn page_is_served_at_its_path() {
        let router = RouterBuilder::new()
            .page(PageFn::new(path("/about"), page_render))
            .build();
        let response = call(&router, Method::GET, "/about");
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_str(&response), "PAGE");
        assert_eq!(
            response.headers().get(http::header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
    }

    #[test]
    fn matching_layouts_nest_from_least_to_most_specific() {
        let router = RouterBuilder::new()
            .page(PageFn::new(path("/settings/profile"), page_render))
            .layout(LayoutFn::new(path("/"), wrapping_layout!("root")))
            .layout(LayoutFn::new(
                path("/settings"),
                wrapping_layout!("settings"),
            ))
            .build();

        let response = call(&router, Method::GET, "/settings/profile");
        assert_eq!(body_str(&response), "root(settings(PAGE))");
    }

    #[test]
    fn layout_only_wraps_pages_under_its_prefix() {
        let router = RouterBuilder::new()
            .page(PageFn::new(path("/about"), page_render))
            .layout(LayoutFn::new(
                path("/settings"),
                wrapping_layout!("settings"),
            ))
            .build();

        // The /settings layout does not apply to /about.
        assert_eq!(body_str(&call(&router, Method::GET, "/about")), "PAGE");
    }

    // ── method_not_allowed helper ──

    #[test]
    fn method_not_allowed_lists_supported_methods() {
        let mut endpoint = Endpoint::default();
        endpoint.insert(Method::GET, 0);
        endpoint.insert(Method::POST, 1);

        let response = method_not_allowed(&endpoint);
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

        let allow = response
            .headers()
            .get(http::header::ALLOW)
            .unwrap()
            .to_str()
            .unwrap();
        assert!(allow.contains("GET"), "allow header was {allow:?}");
        assert!(allow.contains("POST"), "allow header was {allow:?}");
    }
}
