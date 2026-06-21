use std::any::Any;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use http::{HeaderValue, Method, StatusCode};
use topcoat_core::runtime::context::{ContextMap, Cx};

use crate::runtime::{
    Body, Endpoint, Layer, LayoutFn, Next, PageFn, PageWithLayouts, RawPathParams, Request,
    Response, Route, finalize, not_found, respond,
};

/// A finalized collection of [`Route`]s, ready to dispatch requests.
///
/// A `Router` is produced from a [`RouterBuilder`] via
/// [`build`](RouterBuilder::build); start one with [`Router::builder`].
/// Dispatch a request to its matching route with [`handle`](Self::handle).
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
        // least- to most-specific so the outermost layer runs first.
        let route = &*self.routes[index];
        let route_path = route.path();
        let mut layers: Vec<&dyn Layer> = self
            .layers
            .iter()
            .map(|layer| &**layer)
            .filter(|layer| route_path.starts_with(&layer.path()))
            .collect();
        layers.sort_by_key(|layer| layer.path().len());

        let mut cx = Cx::new(self.app_context.clone(), ContextMap::new());
        cx.insert(path_params);
        cx.insert(parts);

        let next = Next::new(&layers, route);
        let response = respond(next.run(&mut cx, body).await);
        finalize(&cx, response)
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

/// Collects [`Route`]s before they are finalized into a [`Router`].
///
/// Register routes with [`route`](Self::route), then call
/// [`build`](Self::build) to produce the immutable [`Router`].
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::router::Router;
///
/// pub fn router() -> Router {
///     Router::builder()
///         .route(api_health)
///         .route(create_user)
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
    #[cfg(feature = "discover")]
    pub fn discover_layouts(mut self) -> Self {
        for layout in inventory::iter::<LayoutFn>().cloned() {
            self = self.layout(layout);
        }
        self
    }

    /// Registers a [`Layer`] that wraps every matched route whose path begins
    /// with the layer's path, like a layout.
    ///
    /// When several layers match a route they nest from least-specific
    /// (outermost) to most-specific (innermost).
    pub fn layer(mut self, layer: impl Layer) -> Self {
        self.layers.push(Box::new(layer));
        self
    }

    /// Registers every layer annotated with `#[layer]` and collected at link
    /// time.
    #[cfg(feature = "discover")]
    pub fn discover_layers(mut self) -> Self {
        for layer in inventory::iter::<crate::runtime::LayerFn>().cloned() {
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
    /// Each [`PageFn`] is matched against the registered [`LayoutFn`]s by path
    /// prefix and wired into a [`PageWithLayouts`], which is registered as a
    /// `GET` route alongside the explicit routes.
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
                .filter(|layout| page.path().starts_with(layout.path()))
                .cloned()
                .collect();
            matching.sort_by_key(|layout| layout.path().len());
            routes.push(Box::new(PageWithLayouts::new(page, matching)));
        }

        // Group routes that share a path into a single endpoint first, since
        // matchit rejects inserting the same path twice.
        let mut grouped: HashMap<Cow<'static, str>, Endpoint> = HashMap::new();
        for (index, route) in routes.iter().enumerate() {
            grouped
                .entry(route.path().to_matchit_path())
                .or_default()
                .insert(route.method(), index);
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
