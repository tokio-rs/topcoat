use std::{
    any::{Any, type_name},
    borrow::Cow,
    collections::HashMap,
    sync::Arc,
};

use bumpalo::Bump;
use topcoat_core::context::{ContextMap, ContextValues};

use crate::{
    Endpoint, Handler, Layer, Layout, Methods, Page, Path, RegisteredRoute, Route, RouteEntry,
    Router, TrailingSlash, TrustedProxies,
    arena::{Objects, RouterCell, Staged, Tables},
    layers_for_path, layouts_for_path,
};

/// Builds a [`Router`] for a Topcoat application.
///
/// Register handlers directly or through discovery, configure the application,
/// then call [`build`](Self::build).
///
/// Integrations can provide extension traits with additional setup methods.
///
/// # Examples
///
/// ```rust
/// # struct AppConfig;
/// # impl AppConfig { fn load() -> Self { Self } }
/// use topcoat::{
///     asset::{AssetBundle, RouterBuilderAssetExt},
///     cookie::CookieLayer,
///     router::{Router, RouterBuilderDiscoverExt},
/// };
///
/// pub fn router() -> Router {
///     Router::builder()
///         .discover()
///         .layer(CookieLayer::new())
///         .assets(AssetBundle::load().unwrap())
///         .with(AppConfig::load())
///         .build()
/// }
/// ```
pub struct RouterBuilder {
    /// The router's arena, holding the routes, pages, layouts and layers
    /// registered so far.
    staged: Staged,
    context: ContextMap,
    trusted_proxies: TrustedProxies,
    trailing_slash: TrailingSlash,
}

impl RouterBuilder {
    /// Creates an empty builder with no routes registered.
    #[must_use]
    pub fn new() -> Self {
        Self {
            staged: Staged::default(),
            context: ContextMap::new(),
            trusted_proxies: TrustedProxies::new(),
            trailing_slash: TrailingSlash::default(),
        }
    }

    /// Returns `true` if no routes, pages, or layouts have been registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        let objects = self.staged.borrow_dependent();
        objects.routes.is_empty() && objects.pages.is_empty() && objects.layouts.is_empty()
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
        self.staged
            .with_dependent_mut(|arena, objects| objects.routes.push(arena, route));
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
        self.staged
            .with_dependent_mut(|arena, objects| objects.pages.push(arena, page));
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
        self.staged
            .with_dependent_mut(|arena, objects| objects.layouts.push(arena, layout));
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
        self.staged
            .with_dependent_mut(|arena, objects| objects.layers.push(arena, layer));
        self
    }

    /// Registers every layer annotated with `#[layer]` and collected at link
    /// time.
    ///
    /// Discovered layers must have unique paths because discovery does not
    /// define their order. To stack layers at one path, register them explicitly
    /// with [`layer`](Self::layer).
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

    /// Configures which reverse proxies can report the client's IP address.
    ///
    /// By default, [`client_ip`](crate::request::client_ip) returns the IP address of
    /// the direct connection. Behind a reverse proxy, that is the proxy's
    /// address. Use this method to trust your proxies so Topcoat can read the
    /// client's address from their HTTP headers. See [`TrustedProxies`] for
    /// how to choose the proxies and header to use.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use topcoat::router::{Router, TrustedProxies};
    ///
    /// let router = Router::builder()
    ///     .trusted_proxies(TrustedProxies::new().networks(["10.0.0.0/8"]))
    ///     .build();
    /// ```
    #[must_use]
    pub fn trusted_proxies(mut self, trusted_proxies: TrustedProxies) -> Self {
        self.trusted_proxies = trusted_proxies;
        self
    }

    /// Configures how a request for the other trailing-slash form of a
    /// route's path is handled.
    ///
    /// By default such a request is redirected to the form the route
    /// declares: `/users/` to a page at `/users`, and `/users` to a page at
    /// `/users/`. See [`TrailingSlash`] for the other policies.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use topcoat::router::{Router, TrailingSlash};
    ///
    /// let router = Router::builder()
    ///     .trailing_slash(TrailingSlash::Serve)
    ///     .build();
    /// ```
    #[must_use]
    pub fn trailing_slash(mut self, trailing_slash: TrailingSlash) -> Self {
        self.trailing_slash = trailing_slash;
        self
    }

    /// Adds a value that all requests to this router can share.
    ///
    /// Read it with [`context`](topcoat_core::context::context). Adding the
    /// same type again on the builder shadows its value, like
    /// [`Cx::with`](topcoat_core::context::Cx::with) does during a request.
    /// Reads return the newest value, and a child context can shadow it
    /// without changing the router's value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # struct User;
    /// use topcoat::{
    ///     context::{Cx, context},
    ///     router::{Router, module_router},
    /// };
    ///
    /// struct Database {/* ... */}
    /// # impl Database {
    /// #     fn connect() -> Self { Self {} }
    /// #     async fn fetch_user(&self, _id: u64) -> User { User }
    /// # }
    ///
    /// pub fn router() -> Router {
    ///     module_router!().with(Database::connect()).build()
    /// }
    ///
    /// async fn fetch_user(cx: &Cx, id: u64) -> User {
    ///     let db: &Database = context(cx);
    ///     db.fetch_user(id).await
    /// }
    /// ```
    #[must_use]
    pub fn with<T>(mut self, value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        self.context.insert(value);
        self
    }

    /// Adds several values for requests to share.
    ///
    /// Pass a tuple or a [`ContextMap`]. Each value is registered as if it
    /// had been passed to [`with`](Self::with).
    #[must_use]
    pub fn with_many<V>(mut self, values: V) -> Self
    where
        V: ContextValues,
    {
        values.install(&mut self.context);
        self
    }

    /// Borrows the value of type `T` already added to this builder.
    ///
    /// # Panics
    ///
    /// Panics if the builder has no value of type `T`.
    #[must_use]
    #[track_caller]
    pub fn context<T>(&self) -> &T
    where
        T: Any + Send + Sync,
    {
        match self.try_context() {
            Some(value) => value,
            None => panic!(
                "attempted to access context of type `{:?}`, but this type was not registered on the router",
                type_name::<T>()
            ),
        }
    }

    /// Borrows the value of type `T` already added to this builder, or returns
    /// `None` if it is absent.
    #[must_use]
    pub fn try_context<T>(&self) -> Option<&T>
    where
        T: Any + Send + Sync,
    {
        self.context.get::<T>()
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
        let cell = RouterCell::build(self.staged, |arena, objects, contexts| {
            tables(
                arena,
                objects,
                contexts,
                self.context,
                self.trusted_proxies,
                self.trailing_slash,
            )
        });
        Router::new(cell)
    }
}

impl Default for RouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Builds the tables of a router in `arena`, pointing at the values in
/// `objects`. The context of each route and page is pushed to `contexts`.
///
/// # Panics
///
/// Panics if two routes are ambiguous, if a route declares no methods, if
/// two paths conflict, or if a layer wraps no route.
fn tables<'r>(
    arena: &'r Bump,
    objects: &'r Objects<'r>,
    contexts: &'r mut Vec<Arc<ContextMap>>,
    context: ContextMap,
    trusted_proxies: TrustedProxies,
    trailing_slash: TrailingSlash,
) -> Tables<'r> {
    let all_layers: Vec<&'r dyn Layer> = objects.layers.iter().collect();
    let all_layouts: Vec<&'r dyn Layout> = objects.layouts.iter().collect();
    // Each layer's and layout's values are built once and shared by
    // everything it wraps.
    let layer_contexts: Vec<ContextMap> = all_layers.iter().map(|&layer| layer.context()).collect();
    let layout_contexts: Vec<ContextMap> = all_layouts
        .iter()
        .map(|&layout| layout.context())
        .collect();

    // Each distinct layer stack and layout list is stored once and shared by
    // the routes that use it.
    let mut layer_stacks: HashMap<Vec<usize>, &'r [&'r dyn Layer]> = HashMap::new();
    let mut layout_lists: HashMap<Vec<usize>, &'r [&'r dyn Layout]> = HashMap::new();
    let mut layers_used = vec![false; all_layers.len()];

    // Every route and page, with the positions of the layouts wrapping it
    // and its own values.
    let routes = objects
        .routes
        .iter()
        .map(|route| (Handler::Route(route), Vec::new(), route.context()));
    let pages = objects.pages.iter().map(|page| {
        let positions = layouts_for_path(&all_layouts, page.path());
        let layouts = *layout_lists
            .entry(positions.clone())
            .or_insert_with_key(|positions| {
                &*arena.alloc_slice_fill_iter(positions.iter().map(|&layout| all_layouts[layout]))
            });
        (Handler::Page { page, layouts }, positions, page.context())
    });

    // The route entries point at their contexts, so every context is created
    // before the first entry.
    let mut pending = Vec::new();
    for (handler, layouts, own_context) in routes.chain(pages) {
        // The layers wrapping a route are selected against its own path,
        // group segments included, so a layer inside a group wraps only the
        // routes registered through that group.
        let layers = layers_for_path(&all_layers, handler.path());

        // Outermost layer first and the route last, so each level shadows
        // the levels around it. Layers run outside layouts, so a route's
        // layout values shadow its layer values.
        let mut context = ContextMap::new();
        for &layer in &layers {
            layers_used[layer] = true;
            layer_contexts[layer].clone().install(&mut context);
        }
        for &layout in &layouts {
            layout_contexts[layout].clone().install(&mut context);
        }
        own_context.install(&mut context);
        let context = (!context.is_empty()).then(|| {
            contexts.push(Arc::new(context));
            contexts.len() - 1
        });

        let layers = *layer_stacks.entry(layers).or_insert_with_key(|layers| {
            &*arena.alloc_slice_fill_iter(layers.iter().map(|&layer| all_layers[layer]))
        });
        pending.push((handler, layers, context));
    }
    let contexts: &'r [Arc<ContextMap>] = contexts;

    // Group routes that share a path onto a single endpoint. Two routes that
    // resolve to the same path *and* method are ambiguous, so reject them
    // here. The grouping map keys endpoints by their matchit path so that
    // routes differing only in their group segments agree on the one their
    // endpoint serves.
    let mut endpoints: Vec<(Cow<'static, str>, Endpoint<'r>)> = Vec::new();
    let mut grouped: HashMap<Cow<'static, str>, usize> = HashMap::new();
    let mut declared = Vec::with_capacity(pending.len());
    for (handler, layers, context) in pending {
        let entry: &'r RouteEntry<'r> = arena.alloc(RouteEntry {
            handler,
            layers,
            context: context.map(|context| &contexts[context]),
        });

        let path = handler.path();
        let index = *grouped
            .entry(path.to_matchit_path())
            .or_insert_with_key(|served| {
                let served_path = Path::new(arena.alloc_str(served));
                endpoints.push((served.clone(), Endpoint::new(served_path)));
                endpoints.len() - 1
            });
        let endpoint = &mut endpoints[index].1;

        // An any-method route shares its path with specific-method routes
        // (which win at dispatch), so the two kinds are checked for
        // duplicates independently.
        match handler.methods() {
            Methods::Any => {
                assert!(
                    endpoint.any().is_none(),
                    "duplicate any-method route registered for `{}`",
                    endpoint.path()
                );
                endpoint.insert_any(entry);
            }
            Methods::Only(methods) => {
                assert!(!methods.is_empty(), "route `{path}` registers no methods");
                for method in methods {
                    assert!(
                        endpoint.get(method).is_none(),
                        "duplicate route registered for `{method} {}`",
                        endpoint.path()
                    );
                    endpoint.insert(arena, method, entry);
                }
            }
        }
        declared.push(RegisteredRoute::new(endpoint.path(), entry));
    }

    // Sanity check for unused layers. Layers without a path always run, and
    // the root path prefixes every route path, so neither can be a mistyped
    // path; only narrower paths can go unused.
    for (&layer, used) in all_layers.iter().zip(layers_used) {
        if let Some(path) = layer.path() {
            assert!(
                used || path == Path::ROOT,
                "layer with path `{path}` did not match any route, this is likely a mistake"
            );
        }
    }

    // Layers without a path wrap every request, so requests that matched no
    // route still run them. Among them the most recently registered runs
    // first, like layers sharing a path.
    let always_layers: Vec<&'r dyn Layer> = all_layers
        .iter()
        .copied()
        .filter(|&layer| layer.path().is_none())
        .rev()
        .collect();
    let always_layers: &'r [&'r dyn Layer] = arena.alloc_slice_copy(&always_layers);

    let mut matcher = matchit::Router::new();
    let mut served = Vec::with_capacity(endpoints.len());
    for (path, mut endpoint) in endpoints {
        endpoint.alias_head_to_get();
        match matcher.insert(path, endpoint) {
            Ok(()) => served.push(endpoint),
            Err(matchit::InsertError::Conflict { with }) => {
                panic!("failed to register route: conflicts with registered route `{with}`")
            }
            Err(error) => panic!("failed to register route: {error}"),
        }
    }

    // The twins are the router's own routes, so they are not declared.
    trailing_slash.register_twins(arena, &mut matcher, &served, always_layers);

    Tables {
        matcher,
        always_layers,
        declared: arena.alloc_slice_copy(&declared),
        context: Arc::new(context),
        trusted_proxies,
    }
}

#[cfg(test)]
mod tests {
    use topcoat_core::context::Cx;
    use topcoat_view::BoxView;

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
    fn render_page(_cx: &Cx, _body: Body) -> BoxView<'static> {
        Box::pin(())
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
    fn a_later_value_shadows_an_earlier_one() {
        let builder = RouterBuilder::new().with(1u32).with_many((2u32, Greeting));

        assert_eq!(*builder.context::<u32>(), 2);
        assert!(builder.try_context::<Greeting>().is_some());
        assert!(builder.try_context::<u64>().is_none());
    }
}
