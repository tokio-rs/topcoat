use std::any::{Any, type_name};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use topcoat_core::context::{ContextMap, CxBuilder};

use crate::{
    Endpoint, Layer, LayerId, Layers, LayoutFn, Next, PageFn, PageWithLayouts, PathSegment,
    RawPathParams, Request, Response, Route, Terminal, respond,
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
    /// The layers registered on this router, wrapping matched routes by path
    /// prefix.
    layers: Layers,
    /// The values shared by every request, read back via
    /// [`app_context`](topcoat_core::context::app_context).
    app_context: Arc<ContextMap>,
    /// The compression applied to responses on their way out.
    #[cfg(feature = "compression")]
    compression: crate::Compression,
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
        // reuses its endpoint's precomputed layer stack, whether the method
        // matches (a route) or not (405), so both flow through the same layers.
        // An unmatched path (404) has no precomputed stack, so its layers are
        // selected from the request path on this cold path.
        let not_found_layers: Vec<LayerId>;
        let (layers, terminal, path_params) =
            if let Ok(matched) = self.endpoints.at(parts.uri.path()) {
                let endpoint = matched.value;
                let path_params = {
                    debug_assert_eq!(endpoint.path_params().len(), matched.params.len());
                    let keys = endpoint.path_params().iter().cloned();
                    let values = matched.params.iter().map(|(_, value)| value);
                    RawPathParams::from_pairs(keys.zip(values))
                };
                let terminal = match endpoint.get(&parts.method) {
                    Some(index) => Terminal::Route(&*self.routes[index]),
                    None => Terminal::MethodNotAllowed(matched.value),
                };
                (matched.value.layers(), terminal, path_params)
            } else {
                not_found_layers = self.layers.match_path(parts.uri.path());
                (
                    &*not_found_layers,
                    Terminal::NotFound,
                    RawPathParams::default(),
                )
            };

        let mut cx = CxBuilder::new(self.app_context.clone());
        cx.insert(path_params);
        cx.insert(parts);

        let next = Next::new(&self.layers, layers, terminal);
        let response = next.run(&mut cx, body).await;
        let response = respond(&cx, response);

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
    layers: Layers,
    context: ContextMap,
    #[cfg(feature = "compression")]
    compression: crate::Compression,
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
            layers: Layers::default(),
            context,
            #[cfg(feature = "compression")]
            compression: crate::Compression::new(),
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
        for route in inventory::iter::<crate::RouteFn>().cloned() {
            self = self.route(route);
        }
        self
    }

    /// Registers a page: anything convertible into a [`PageFn`], like the
    /// marker `#[page]` generates. Order doesn't matter: layout matching is
    /// based on path prefixes, not registration order.
    #[must_use]
    pub fn page(mut self, page: impl Into<PageFn>) -> Self {
        self.pages.push(page.into());
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

    /// Registers a layout: anything convertible into a [`LayoutFn`], like the
    /// marker `#[layout]` generates. A layout applies to every page whose path
    /// starts with the layout's path prefix.
    #[must_use]
    pub fn layout(mut self, layout: impl Into<LayoutFn>) -> Self {
        self.layouts.push(layout.into());
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
        let mut seen = std::collections::HashSet::<crate::PathBuf>::new();
        for layout in inventory::iter::<LayoutFn>().cloned() {
            assert!(
                seen.insert(layout.path().to_owned()),
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
        let mut seen = std::collections::HashSet::<crate::PathBuf>::new();
        for layer in inventory::iter::<crate::LayerFn>().cloned() {
            assert!(
                seen.insert(layer.path().to_owned()),
                "multiple discovered layers registered for the same path \"{}\"",
                layer.path()
            );
            self = self.layer(layer);
        }
        self
    }

    /// Configures the compression applied to responses.
    ///
    /// By default the router compresses each response with the algorithm
    /// negotiated from the request's `Accept-Encoding` header. Pass
    /// [`Compression::off`](crate::Compression::off) to disable compression
    /// (say, behind a reverse proxy that compresses already), or a tuned
    /// [`Compression`](crate::Compression) value to adjust it.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use topcoat::router::{Compression, Router};
    ///
    /// let router = Router::builder().compression(Compression::off()).build();
    /// ```
    #[cfg(feature = "compression")]
    #[must_use]
    pub fn compression(mut self, compression: crate::Compression) -> Self {
        self.compression = compression;
        self
    }

    /// Registers a unique value that is accessible to every request sent to
    /// this router by its type `T`. The top-level
    /// [`app_context`](topcoat_core::context::app_context) function can be used to
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
        assert!(
            self.context.insert(value).is_none(),
            "duplicate context entry for type `{:?}`",
            type_name::<T>()
        );
        self
    }

    /// Returns a reference to the app context value of type `T` registered with
    /// [`app_context`](Self::app_context), or `None` if none has been
    /// registered.
    ///
    /// Lets code that registers a shared value lazily check for it first, rather
    /// than tripping the duplicate-registration panic on a second call.
    #[must_use]
    pub fn get_app_context<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        self.context.get::<T>()
    }

    /// Returns a mutable reference to the app context value of type `T`
    /// registered with [`app_context`](Self::app_context), or `None` if none
    /// has been registered.
    #[must_use]
    pub fn get_app_context_mut<T>(&mut self) -> Option<&mut T>
    where
        T: Any + Send + Sync,
    {
        self.context.get_mut::<T>()
    }

    /// Finalizes the registered routes, pages, and layouts into a [`Router`].
    ///
    /// # Panics
    ///
    /// Panics if two routes resolve to the same path and HTTP method, since the
    /// router would have no way to choose between them.
    ///
    /// Also panics if two routes resolve to the same path but different layers
    /// wrap them (possible when their group segments differ, e.g. `/(a)/x` and
    /// `/(b)/x` with a layer at `/(a)`): every route at a path shares one layer
    /// stack, so the divergence is rejected rather than resolved by
    /// registration order.
    #[must_use]
    pub fn build(self) -> Router {
        let RouterBuilder {
            mut routes,
            pages,
            layouts,
            layers,
            context,
            #[cfg(feature = "compression")]
            compression,
        } = self;

        // Wire each page to the layouts whose path is a prefix of the page's,
        // ordered from least- to most-specific so the page nests innermost.
        for page in pages {
            let mut matching: Vec<LayoutFn> = layouts
                .iter()
                .filter(|layout| page.path().starts_with(layout.path()))
                .cloned()
                .collect();
            matching.sort_by_key(|layout| layout.path().len());
            routes.push(Box::new(PageWithLayouts::new(page, matching)));
        }

        // Group routes that share a path into a single endpoint first, since
        // matchit rejects inserting the same path twice. Two routes that resolve
        // to the same path *and* method are ambiguous, so reject them here.
        // Remember each group's first route index to name it in the layer
        // divergence panic below.
        let mut grouped: HashMap<Cow<'static, str>, (usize, Endpoint)> = HashMap::new();
        let mut interned_path_params: HashMap<&str, Arc<str>> = HashMap::new();
        for (index, route) in routes.iter().enumerate() {
            let layer_stack = layers.for_endpoint(route.path());
            let (first, endpoint) = grouped
                .entry(route.path().to_matchit_path())
                .or_insert_with(|| {
                    let path_params = route
                        .path()
                        .segments()
                        .filter_map(|segment| match segment {
                            // A catch-all is captured by matchit like a param,
                            // so it needs a key here too.
                            PathSegment::Param(param) | PathSegment::CatchAll(param) => {
                                let interned =
                                    interned_path_params.entry(param).or_insert_with(|| {
                                        Arc::from(param.to_owned().into_boxed_str())
                                    });
                                Some(interned.clone())
                            }
                            _ => None,
                        })
                        .collect();
                    let endpoint =
                        Endpoint::new(path_params, layer_stack.clone().into_boxed_slice());
                    (index, endpoint)
                });

            // Every route grouped at a path shares the endpoint's layer stack
            // (the 405 fallback included), so their full paths -- group
            // segments and all -- must select the same layers. Reject a
            // divergence rather than let registration order pick a winner.
            assert!(
                layer_stack == endpoint.layers(),
                "routes `{}` and `{}` serve the same URL path but select different layers",
                routes[*first].path(),
                route.path(),
            );

            let method = route.method();
            assert!(
                endpoint.get(&method).is_none(),
                "duplicate route registered for `{method} {}`",
                route.path().to_matchit_path()
            );
            endpoint.insert(method, index);
        }

        // Precompute the layer stack wrapping each endpoint once, so dispatch
        // only has to index into it rather than filter and sort per request.
        let mut endpoints = matchit::Router::new();
        for (path, (_, mut endpoint)) in grouped {
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
            #[cfg(feature = "compression")]
            compression,
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
    use topcoat_core::context::{Cx, app_context, request_context};
    use topcoat_core::error::Result;
    use topcoat_view::{HtmlContext, PartsWriter, View, ViewParts};

    use super::*;
    use crate::{
        Body, Bytes, IntoResponse, LayerFn, LayerFuture, Method, Path, RouteFn, RouteFuture, Slot,
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
        let mut parts = ViewParts::new();
        PartsWriter::new(&mut parts, HtmlContext::Text).push_str(text);
        View::new(parts)
    }

    fn render_page(_cx: &Cx, _body: Body) -> ViewFuture<'_> {
        Box::pin(async move { Ok(view("page")) })
    }

    /// Wraps the child content in `R[ ... ]` so layout nesting is observable.
    fn layout_root<'cx>(_cx: &'cx Cx, slot: Slot<'cx>) -> ViewFuture<'cx> {
        Box::pin(async move {
            let inner = slot.await?;
            let mut parts = ViewParts::new();
            PartsWriter::new(&mut parts, HtmlContext::Text).push_str("R[");
            parts.push_view(inner);
            PartsWriter::new(&mut parts, HtmlContext::Text).push_str("]");
            Ok(View::new(parts))
        })
    }

    /// Wraps the child content in `A[ ... ]`.
    fn layout_admin<'cx>(_cx: &'cx Cx, slot: Slot<'cx>) -> ViewFuture<'cx> {
        Box::pin(async move {
            let inner = slot.await?;
            let mut parts = ViewParts::new();
            PartsWriter::new(&mut parts, HtmlContext::Text).push_str("A[");
            parts.push_view(inner);
            PartsWriter::new(&mut parts, HtmlContext::Text).push_str("]");
            Ok(View::new(parts))
        })
    }

    // -- RouterBuilder --

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
        // The catch-all captures the remainder of the URL, slashes included.
        let (_, _, body) = send(&router, Method::GET, "/files/a/b/c");
        assert_eq!(&body[..], b"rest=a/b/c");
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

    #[test]
    fn layers_run_for_trailing_slash_urls() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/admin/x"), say_route))
                .layer(LayerFn::new(path("/admin"), trace_admin))
                .layer(LayerFn::new(path("/"), trace_root)),
        );

        // A trailing slash is a different URL: the route does not match, but
        // the layers wrapping the path still run around the 404.
        let (status, _, _) = send(&router, Method::GET, "/admin/x/");
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(*trace.lock().unwrap(), vec!["root", "admin"]);
    }

    #[test]
    fn layers_do_not_run_for_lookalike_prefixes() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/admin/x"), say_route))
                .layer(LayerFn::new(path("/admin"), trace_admin))
                .layer(LayerFn::new(path("/"), trace_root)),
        );

        // `/admin` prefixes the string `/administrator` but not its segments,
        // so only the root layer wraps the 404.
        let (status, _, _) = send(&router, Method::GET, "/administrator");
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(*trace.lock().unwrap(), vec!["root"]);
    }

    #[test]
    fn layer_selection_ignores_query_strings() {
        let (router, trace) = trace_router(
            RouterBuilder::new()
                .route(RouteFn::new(Method::GET, path("/admin/x"), say_route))
                .layer(LayerFn::new(path("/admin"), trace_admin)),
        );

        let (status, _, _) = send(&router, Method::GET, "/admin/x?tab=users");
        assert_eq!(status, StatusCode::OK);
        assert_eq!(*trace.lock().unwrap(), vec!["admin"]);

        // The same holds on the 404 path, which selects layers by URL.
        trace.lock().unwrap().clear();
        let (status, _, _) = send(&router, Method::GET, "/admin/missing?tab=users");
        assert_eq!(status, StatusCode::NOT_FOUND);
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
    fn group_layers_wrap_not_found_urls_anywhere() {
        let (router, trace) =
            trace_router(RouterBuilder::new().layer(LayerFn::new(path("/(auth)"), trace_auth)));

        // A 404 URL cannot be attributed to a group, and a group-only path is
        // URL-equivalent to the root, so the layer wraps every unmatched URL.
        let (status, _, _) = send(&router, Method::GET, "/missing");
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(*trace.lock().unwrap(), vec!["auth"]);
    }

    #[test]
    #[should_panic(expected = "select different layers")]
    fn routes_sharing_a_url_with_different_layers_panic() {
        // Both routes serve `/x` (groups are stripped from the URL), but only
        // the `(a)` route is wrapped by the `/(a)` layer. The endpoint's layer
        // stack is shared, so the build rejects the divergence.
        let _ = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/(a)/x"), say_route))
            .route(RouteFn::new(Method::POST, path("/(b)/x"), say_posted))
            .layer(LayerFn::new(path("/(a)"), trace_auth))
            .build();
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
