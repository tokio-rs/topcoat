use std::{
    any::{Any, type_name},
    borrow::Cow,
    collections::HashMap,
    fmt,
    sync::Arc,
};

use topcoat_core::{base_url::BaseUrl, context::AppContext};

use crate::{
    Endpoint, EndpointIndex, Endpoints, Layer, Layout, Methods, OriginLayer, OriginPolicy, Page,
    PageWithLayouts, Path, Route, Router, RouterInner, Routes, layers_for_path,
};

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
    pages: Vec<Box<dyn Page>>,
    layouts: Vec<Arc<dyn Layout>>,
    layers: Vec<Arc<dyn Layer>>,
    context: AppContext,
    origin_policy: OriginPolicy,
    #[cfg(feature = "compression")]
    compression: crate::Compression,
}

impl RouterBuilder {
    /// Creates an empty builder with no routes registered.
    #[must_use]
    pub fn new() -> Self {
        let mut context = AppContext::new();
        // Register `()` so APIs generic over an app context type can default to `S = ()`.
        context.insert(());
        Self {
            routes: Vec::new(),
            pages: Vec::new(),
            layouts: Vec::new(),
            layers: Vec::new(),
            context,
            origin_policy: OriginPolicy::new(),
            #[cfg(feature = "compression")]
            compression: crate::Compression::new(),
        }
    }

    /// Returns `true` if no routes, pages, or layouts have been registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.routes.is_empty() && self.pages.is_empty() && self.layouts.is_empty()
    }

    /// Registers a [`Route`], an HTTP handler bound to a set of methods and a
    /// path.
    ///
    /// A route responds to the methods its [`Route::methods`] declares: one or
    /// more specific methods, or every method via [`Methods::Any`]. Specific-
    /// method routes and one any-method route can share a path; the specific
    /// method wins at dispatch.
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
        for &route in inventory::iter::<&'static dyn Route>() {
            self = self.route(route);
        }
        self
    }

    /// Registers a [`Page`], like the marker `#[page]` generates. Order
    /// doesn't matter: layout matching is based on path prefixes, not
    /// registration order.
    ///
    /// A page serves the methods its [`Page::methods`] declares (`GET` unless
    /// the page opts into others).
    #[must_use]
    pub fn page(mut self, page: impl Page) -> Self {
        self.pages.push(Box::new(page));
        self
    }

    /// Registers every [`Page`] annotated with `#[page]` and collected at
    /// link time.
    #[cfg(feature = "discover")]
    #[must_use]
    pub fn discover_pages(mut self) -> Self {
        for &page in inventory::iter::<&'static dyn Page>() {
            self = self.page(page);
        }
        self
    }

    /// Registers a [`Layout`], like the marker `#[layout]` generates. A
    /// layout applies to every page whose path starts with the layout's path
    /// prefix.
    #[must_use]
    pub fn layout(mut self, layout: impl Layout) -> Self {
        self.layouts.push(Arc::new(layout));
        self
    }

    /// Registers every [`Layout`] annotated with `#[layout]` and collected at
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
    #[track_caller]
    pub fn discover_layouts(mut self) -> Self {
        let mut seen = std::collections::HashSet::<crate::PathBuf>::new();
        for &layout in inventory::iter::<&'static dyn Layout>() {
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
    /// with the layer's path, like a layout. A layer without a path
    /// ([`Layer::path`] returns `None`) wraps every request, including one
    /// that matches no route (a 404 or 405).
    ///
    /// When layers at *different* paths match a route they nest from
    /// least-specific (outermost) to most-specific (innermost), with pathless
    /// layers outside them all. Multiple layers may share the same path; among
    /// those, the most recently registered runs first (outermost), so
    /// `.layer(a).layer(b)` runs `b` around `a` when both sit at the same
    /// path.
    #[must_use]
    pub fn layer(mut self, layer: impl Layer) -> Self {
        self.layers.push(Arc::new(layer));
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
    #[track_caller]
    pub fn discover_layers(mut self) -> Self {
        let mut seen = std::collections::HashSet::<Option<crate::PathBuf>>::new();
        for &layer in inventory::iter::<&'static dyn Layer>() {
            assert!(
                seen.insert(layer.path().map(Path::to_owned)),
                "multiple discovered layers registered for the same path \"{}\"",
                layer.path().map_or("<none>", Path::as_str)
            );
            self = self.layer(layer);
        }
        self
    }

    /// Registers the [`OriginPolicy`] the router applies to every request.
    ///
    /// The default policy denies state-changing cross-origin browser requests
    /// and cross-origin WebSocket handshakes; pass a policy to trust
    /// cross-origin peers, exempt individual routes, or opt out entirely.
    #[must_use]
    pub fn origin_policy(mut self, origin_policy: OriginPolicy) -> Self {
        self.origin_policy = origin_policy;
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

    /// Registers the base URL the application is publicly reachable at, like
    /// `https://example.com`.
    ///
    /// Relative URLs work anywhere within the site, but rendered content
    /// that leaves it (e.g. links and images in emails, feeds, or sitemaps)
    /// needs the absolute form. The base URL is stored on the app context;
    /// read it back with [`base_url`](topcoat_core::base_url::base_url) (or
    /// [`try_base_url`](topcoat_core::base_url::try_base_url)) and resolve
    /// paths against it with
    /// [`BaseUrl::join`](topcoat_core::base_url::BaseUrl::join).
    ///
    /// Accepts anything convertible into a [`BaseUrl`]. A string is parsed,
    /// so it must be an absolute `http` or `https` URL without a query or
    /// fragment, with an optional path prefix for applications mounted
    /// under one.
    ///
    /// # Panics
    ///
    /// Panics if the value is not a valid base URL, or if a base URL has
    /// already been registered.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use topcoat::router::Router;
    ///
    /// let router = Router::builder().base_url("https://example.com").build();
    /// ```
    #[must_use]
    #[track_caller]
    pub fn base_url(self, base_url: impl TryInto<BaseUrl, Error: fmt::Display>) -> Self {
        match base_url.try_into() {
            Ok(base_url) => self.app_context(base_url),
            Err(error) => panic!("{error}"),
        }
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
    /// use topcoat::{
    ///     context::{Cx, app_context},
    ///     router::Router,
    /// };
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
    #[track_caller]
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
    /// Panics if two routes resolve to the same path and HTTP method (or both
    /// respond to every method at the same path), since the router would have
    /// no way to choose between them, and if a route declares an empty method
    /// set, since it could never be dispatched to.
    #[must_use]
    #[track_caller]
    pub fn build(self) -> Router {
        // Wire each page to the layouts whose path is a prefix of the page's,
        // ordered from least- to most-specific so the page nests innermost.
        let mut registrations = self.routes;
        for page in self.pages {
            let mut matching: Vec<Arc<dyn Layout>> = self
                .layouts
                .iter()
                .filter(|layout| page.path().starts_with(layout.path()))
                .cloned()
                .collect();
            matching.sort_by_key(|layout| layout.path().len());
            registrations.push(Box::new(PageWithLayouts::new(page, matching)));
        }

        // Group routes that share a path onto a single endpoint. Two routes
        // that resolve to the same path *and* method are ambiguous, so reject
        // them here. The grouping map keys endpoints by their matchit path so
        // that routes differing only in their group segments agree on the one
        // their endpoint serves.
        let mut routes = Routes::default();
        let mut endpoints = Endpoints::default();
        let mut grouped: HashMap<Cow<'static, str>, EndpointIndex> = HashMap::new();
        let mut layers_used = vec![false; self.layers.len()];
        for route in registrations {
            // The layers wrapping a route are selected against its own path,
            // group segments included, so a layer inside a group wraps only
            // the routes registered through that group.
            let layer_stack = layers_for_path(&self.layers, route.path());
            // Mark layers as used, with the same prefix rule the selection
            // applies.
            for (layer, used) in self.layers.iter().zip(&mut layers_used) {
                *used |= layer
                    .path()
                    .is_none_or(|prefix| route.path().starts_with(prefix));
            }

            let endpoint_index = *grouped
                .entry(route.path().to_matchit_path())
                .or_insert_with_key(|key| {
                    endpoints.push(key.clone(), Endpoint::new(Path::new(key)))
                });

            let route_index = routes.push(route, endpoint_index, layer_stack);
            let route = &routes[route_index].route;
            let endpoint = &mut endpoints[endpoint_index];

            // An any-method route shares its path with specific-method routes
            // (which win at dispatch), so the two kinds are checked for
            // duplicates independently.
            match route.methods() {
                Methods::Any => {
                    assert!(
                        endpoint.any().is_none(),
                        "duplicate any-method route registered for `{}`",
                        route.path().to_matchit_path()
                    );
                    endpoint.insert_any(route_index);
                }
                Methods::Only(methods) => {
                    assert!(
                        !methods.is_empty(),
                        "route `{}` registers no methods",
                        route.path()
                    );
                    for method in methods {
                        assert!(
                            endpoint.get(method).is_none(),
                            "duplicate route registered for `{method} {}`",
                            route.path().to_matchit_path()
                        );
                        endpoint.insert(method.clone(), route_index);
                    }
                }
            }
        }

        // Sanity check for unused layers. Layers without a path always run,
        // so only layers with one can go unused.
        for (layer, used) in self.layers.iter().zip(layers_used) {
            if let Some(path) = layer.path() {
                assert!(
                    used,
                    "layer with path `{path}` did not match any route, this is likely a mistake"
                );
            }
        }

        for &endpoint_index in grouped.values() {
            endpoints[endpoint_index].alias_head_to_get();
        }

        // Layers without a path wrap every request, so requests that matched
        // no route still run them. Among them the most recently registered
        // runs first, like layers sharing a path.
        let always_layers: Box<[Arc<dyn Layer>]> = self
            .layers
            .iter()
            .filter(|layer| layer.path().is_none())
            .rev()
            .cloned()
            .collect();

        Router::new(RouterInner {
            routes,
            endpoints,
            always_layers,
            app_context: Arc::new(self.context),
            origin: OriginLayer::new(self.origin_policy),
            #[cfg(feature = "compression")]
            compression: self.compression,
        })
    }
}

impl Default for RouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::{future::Future, pin::Pin};

    use topcoat_core::{context::Cx, error::Result};
    use topcoat_view::View;

    use super::*;
    use crate::{
        Body, LayerFn, LayerFuture, Method, Next, PageFn, Path, RouteFn, RouteFuture,
        response::IntoResponse,
    };

    fn path(s: &'static str) -> Cow<'static, Path> {
        Cow::Borrowed(Path::new(s))
    }

    /// A stand-in handler; builder tests register routes without running them.
    fn handler(cx: &Cx, _body: Body) -> RouteFuture<'_> {
        Box::pin(async move { "handled".into_response(cx) })
    }

    /// A stand-in page; builder tests register pages without rendering them.
    fn render_page(
        _cx: &Cx,
        _body: Body,
    ) -> Pin<Box<dyn Future<Output = Result<View>> + Send + '_>> {
        Box::pin(async move { Ok(View::empty()) })
    }

    /// A stand-in layer that continues the chain unchanged.
    fn noop_layer<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
        Box::pin(async move { next.run(cx, body).await })
    }

    struct Greeting;

    #[test]
    fn new_builder_is_empty() {
        let builder = RouterBuilder::new();
        assert!(builder.is_empty());
    }

    #[test]
    fn builder_is_not_empty_after_registering_a_route() {
        let builder = RouterBuilder::new().route(RouteFn::new(Method::GET, path("/x"), handler));
        assert!(!builder.is_empty());
    }

    #[test]
    #[should_panic(expected = "duplicate route")]
    fn duplicate_method_and_path_panics_on_build() {
        let _ = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/x"), handler))
            .route(RouteFn::new(Method::GET, path("/x"), handler))
            .build();
    }

    #[test]
    #[should_panic(expected = "duplicate any-method route")]
    fn duplicate_any_method_routes_panic_on_build() {
        let _ = RouterBuilder::new()
            .route(RouteFn::new(Methods::Any, path("/x"), handler))
            .route(RouteFn::new(Methods::Any, path("/x"), handler))
            .build();
    }

    #[test]
    #[should_panic(expected = "duplicate route")]
    fn overlapping_method_sets_panic_on_build() {
        let _ = RouterBuilder::new()
            .route(RouteFn::new(
                &[Method::GET, Method::POST],
                path("/x"),
                handler,
            ))
            .route(RouteFn::new(Method::POST, path("/x"), handler))
            .build();
    }

    #[test]
    #[should_panic(expected = "registers no methods")]
    fn route_without_methods_panics_on_build() {
        let _ = RouterBuilder::new()
            .route(RouteFn::new(Vec::<Method>::new(), path("/x"), handler))
            .build();
    }

    #[test]
    fn layer_wrapping_a_route_builds() {
        let _ = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/admin/users"), handler))
            .layer(LayerFn::new(Some(path("/admin")), noop_layer))
            .build();
    }

    #[test]
    fn layer_wrapping_a_page_builds() {
        // Pages are wired into routes before layers are checked, so a layer
        // over a page's path counts as used.
        let _ = RouterBuilder::new()
            .page(PageFn::new(Method::GET, path("/admin/p"), render_page))
            .layer(LayerFn::new(Some(path("/admin")), noop_layer))
            .build();
    }

    #[test]
    fn root_layer_without_routes_builds() {
        // A layer at the root path wraps whatever the router serves, so it is
        // exempt from the check even with nothing registered.
        let _ = RouterBuilder::new()
            .layer(LayerFn::new(Some(path("/")), noop_layer))
            .build();
    }

    #[test]
    #[should_panic(expected = "layer with path `/admin` did not match any route")]
    fn layer_matching_no_route_panics() {
        let _ = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/x"), handler))
            .layer(LayerFn::new(Some(path("/admin")), noop_layer))
            .build();
    }

    #[test]
    #[should_panic(expected = "layer with path `/admin` did not match any route")]
    fn layer_matching_part_of_a_segment_panics() {
        // Prefix matching compares whole segments, so `/admin` does not wrap
        // `/administrator`.
        let _ = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/administrator"), handler))
            .layer(LayerFn::new(Some(path("/admin")), noop_layer))
            .build();
    }

    #[test]
    fn routes_sharing_a_url_with_diverging_layers_build() {
        // Both routes serve `/x`, but each carries its own layer stack, so
        // the layer inside `(a)` wrapping only one of them is fine.
        let _ = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/(a)/x"), handler))
            .route(RouteFn::new(Method::POST, path("/(b)/x"), handler))
            .layer(LayerFn::new(Some(path("/(a)")), noop_layer))
            .build();
    }

    #[test]
    #[should_panic(expected = "layer with path `/(a)` did not match any route")]
    fn layer_in_an_unused_group_panics() {
        // Groups are part of the logical path: the `(a)` layer does not wrap
        // the route in `(b)`, even though both serve `/x`.
        let _ = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/(b)/x"), handler))
            .layer(LayerFn::new(Some(path("/(a)")), noop_layer))
            .build();
    }

    #[test]
    #[should_panic(expected = "layer with path `/users/{id}` did not match any route")]
    fn layer_with_another_param_name_panics() {
        // Parameter names are part of a segment, so `{id}` does not wrap an
        // endpoint spelled with `{user_id}`.
        let _ = RouterBuilder::new()
            .route(RouteFn::new(
                Method::GET,
                path("/users/{user_id}/posts"),
                handler,
            ))
            .layer(LayerFn::new(Some(path("/users/{id}")), noop_layer))
            .build();
    }

    #[test]
    #[should_panic(expected = "layer with path `/posts` did not match any route")]
    fn one_unused_layer_among_used_ones_panics() {
        let _ = RouterBuilder::new()
            .route(RouteFn::new(Method::GET, path("/users"), handler))
            .layer(LayerFn::new(Some(path("/")), noop_layer))
            .layer(LayerFn::new(Some(path("/users")), noop_layer))
            .layer(LayerFn::new(Some(path("/posts")), noop_layer))
            .build();
    }

    #[test]
    #[should_panic(expected = "duplicate context entry")]
    fn duplicate_app_context_type_panics() {
        let _ = RouterBuilder::new()
            .app_context(Greeting)
            .app_context(Greeting);
    }

    #[test]
    #[should_panic(expected = "invalid base URL")]
    fn invalid_base_url_panics_at_registration() {
        let _ = RouterBuilder::new().base_url("not a url");
    }
}
