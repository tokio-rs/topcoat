#[cfg(feature = "fs")]
mod directory;

use std::{
    any::Any,
    borrow::Cow,
    pin::Pin,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicUsize, Ordering},
    },
};

use bumpalo::Bump;
#[cfg(feature = "fs")]
pub use directory::*;
use topcoat_core::{
    context::{ContextMap, Cx},
    error::Result,
};

use crate::{
    Body, HrefTarget, IntoPath, Layer, Layout, Methods, OwnedMethods, Page, Path,
    response::Response, route, route_path,
};

/// The future returned by [`Route::handle`]: a boxed, `Send` future borrowing
/// the route and its request context.
pub type RouteFuture<'cx> = Pin<Box<dyn Future<Output = Result<Response>> + Send + 'cx>>;

/// The identity of a registered handler.
///
/// Ids are drawn from a process-wide counter with [`new`](RouteId::new), so
/// every handler in an application gets a distinct one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RouteId(usize);

impl RouteId {
    /// Draws the next id from the process-wide counter.
    ///
    /// A handler calls this once and keeps the result as its identity.
    #[must_use]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    /// Returns how many ids were drawn before this one.
    pub(crate) fn number(self) -> usize {
        self.0
    }
}

/// A single routable endpoint: a set of HTTP methods, a URL path, and a
/// handler.
///
/// This is the core primitive a [`Router`](crate::Router) dispatches to.
/// Register any `Route` with [`RouterBuilder::route`](crate::RouterBuilder::route).
pub trait Route: Send + Sync + 'static {
    /// The identity of this route's handler.
    fn id(&self) -> RouteId;

    /// The HTTP methods this route responds to.
    fn methods(&self) -> Methods<'_>;

    /// The URL path this route handles.
    fn path(&self) -> &Path;

    /// The values attached to this route, such as metadata that describes it.
    ///
    /// Defaults to an empty map.
    fn context(&self) -> ContextMap {
        ContextMap::new()
    }

    /// Handles a request, producing a response.
    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx>;

    /// Returns whether this route handles the current request.
    ///
    /// Only the handler is compared, so a route is current for every value its
    /// path parameters take, whatever the request's query or fragment.
    ///
    /// # Panics
    ///
    /// Panics if the request matched no route: either its path matched no
    /// endpoint, or the endpoint holds no route for the request's method.
    fn is_current(&self, cx: &Cx) -> bool {
        route(cx).id() == self.id()
    }
}

impl<R: Route + ?Sized> Route for &'static R {
    fn id(&self) -> RouteId {
        (**self).id()
    }

    fn methods(&self) -> Methods<'_> {
        (**self).methods()
    }

    fn path(&self) -> &Path {
        (**self).path()
    }

    fn context(&self) -> ContextMap {
        (**self).context()
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        (**self).handle(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Route);

/// The async handler function backing a [`RouteFn`].
pub type RouteHandlerFn = for<'cx> fn(cx: &'cx Cx, body: Body) -> RouteFuture<'cx>;

/// A [`Route`] backed by a plain handler function.
///
/// Turns a function into a route without implementing [`Route`] on a struct,
/// pairing it with the methods and path it serves.
#[derive(Debug, Clone)]
pub struct RouteFn {
    /// The identity of this route's handler.
    id: RouteId,
    /// The HTTP methods this route responds to.
    methods: OwnedMethods,
    /// The URL path this route handles.
    path: Cow<'static, Path>,
    /// The handler function that produces the response.
    handle: RouteHandlerFn,
    /// The values attached to this route.
    context: ContextMap,
}

impl RouteFn {
    /// Creates a new route with explicit methods, path, and handler function.
    ///
    /// The methods are anything convertible into [`OwnedMethods`]: a single
    /// [`Method`](crate::Method), a `&'static [Method]`, a `Vec<Method>`, or
    /// [`Methods::Any`] to respond to every method.
    ///
    /// ```rust
    /// use topcoat::{
    ///     context::Cx,
    ///     router::{Body, Method, RouteFn, RouteFuture},
    /// };
    ///
    /// fn handler(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
    ///     Box::pin(async move { unimplemented!() })
    /// }
    ///
    /// let form = RouteFn::new(&[Method::GET, Method::POST], "/form", handler);
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if `path` is a string that is not a well-formed route path.
    #[track_caller]
    pub fn new(
        methods: impl Into<OwnedMethods>,
        path: impl IntoPath,
        handle: RouteHandlerFn,
    ) -> Self {
        Self {
            id: RouteId::new(),
            methods: methods.into(),
            path: path.into_path(),
            handle,
            context: ContextMap::new(),
        }
    }

    /// Adds `value` to the context of this route.
    ///
    /// The value shadows an earlier value of the same type. Read it during
    /// a request with [`context`](topcoat_core::context::context).
    #[must_use]
    pub fn with<T>(mut self, value: T) -> Self
    where
        T: Any + Send + Sync,
    {
        self.context.insert(value);
        self
    }
}

impl Route for RouteFn {
    fn id(&self) -> RouteId {
        self.id
    }

    fn methods(&self) -> Methods<'_> {
        self.methods.as_methods()
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn context(&self) -> ContextMap {
        self.context.clone()
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        (self.handle)(cx, body)
    }
}

impl HrefTarget for RouteFn {
    #[track_caller]
    fn path<'cx>(&self, cx: &'cx Cx) -> &'cx Path {
        match route_path(cx, self.id) {
            Some(path) => path,
            None => panic!(
                "route `{}` is not registered on the router serving this request",
                self.path
            ),
        }
    }
}

/// What a request needs from the route it matched.
///
/// A route entry borrows from the router it belongs to.
pub(crate) struct RouteEntry<'r> {
    /// What produces the response once the layers have run.
    pub(crate) handler: Handler<'r>,
    /// The layers wrapping this route, ordered from least- to most-specific
    /// so the outermost layer runs first.
    pub(crate) layers: &'r [&'r dyn Layer],
    /// The route's context: the values of the layers and layouts that wrap
    /// it, followed by its own. `None` when it holds no values.
    pub(crate) context: Option<&'r Arc<ContextMap>>,
}

/// What produces the response of a [`RouteEntry`].
#[derive(Clone, Copy)]
pub(crate) enum Handler<'r> {
    /// A route, which produces the response itself.
    Route(&'r dyn Route),
    /// A page, rendered inside its layouts.
    Page {
        page: &'r dyn Page,
        /// The layouts wrapping the page, outermost first.
        layouts: &'r [&'r dyn Layout],
    },
    /// Redirects to the requested URL with its trailing slash added or removed.
    Redirect {
        id: RouteId,
        /// The path this redirect is served at.
        path: &'r Path,
    },
}

impl<'r> Handler<'r> {
    /// The identity of the route or page.
    pub(crate) fn id(&self) -> RouteId {
        match *self {
            Self::Route(route) => route.id(),
            Self::Page { page, .. } => page.id(),
            Self::Redirect { id, .. } => id,
        }
    }

    /// The path the route or page was registered at, group segments
    /// included.
    pub(crate) fn path(&self) -> &'r Path {
        match *self {
            Self::Route(route) => route.path(),
            Self::Page { page, .. } => page.path(),
            Self::Redirect { path, .. } => path,
        }
    }

    /// The HTTP methods the route or page responds to.
    pub(crate) fn methods(&self) -> Methods<'r> {
        match *self {
            Self::Route(route) => route.methods(),
            Self::Page { page, .. } => page.methods(),
            Self::Redirect { .. } => Methods::Any,
        }
    }
}

/// A route registered on a [`Router`](crate::Router), with the path and
/// methods it is served at and the values attached to it.
#[derive(Clone, Copy)]
pub struct RegisteredRoute<'router> {
    id: RouteId,
    /// The URL path the route is served at.
    path: &'router Path,
    entry: &'router RouteEntry<'router>,
}

impl<'router> RegisteredRoute<'router> {
    /// Describes the route behind `entry`, which is served at `path`.
    pub(crate) fn new(path: &'router Path, entry: &'router RouteEntry<'router>) -> Self {
        Self {
            id: entry.handler.id(),
            path,
            entry,
        }
    }

    /// The identity of the route's handler.
    #[must_use]
    pub fn id(&self) -> RouteId {
        self.id
    }

    /// The URL path the route is served at.
    ///
    /// Parameters keep their `{name}` form. Group segments are not part of
    /// it, because they never reach the URL.
    #[must_use]
    pub fn path(&self) -> &'router Path {
        self.path
    }

    /// The HTTP methods the route responds to.
    #[must_use]
    pub fn methods(&self) -> Methods<'router> {
        self.entry.handler.methods()
    }

    /// The route's context: the values of the layers and layouts that wrap
    /// it, followed by its own.
    ///
    /// Values registered on the router builder are not part of it. Read them
    /// with [`Router::context`](crate::Router::context).
    #[must_use]
    pub fn context(&self) -> &'router ContextMap {
        static EMPTY: LazyLock<ContextMap> = LazyLock::new(ContextMap::new);
        match self.entry.context {
            Some(context) => context,
            None => &EMPTY,
        }
    }
}

/// The routes an application registered on a router, found by id.
#[derive(Default)]
pub(crate) struct Routes<'r> {
    /// Every route, sorted by id.
    sorted: &'r [RegisteredRoute<'r>],
    /// The routes indexed by id, with one slot per id starting at the id
    /// numbered `first`. Empty when the ids are too far apart for a table.
    sequential: &'r [Option<&'r RegisteredRoute<'r>>],
    /// The number of the id the first slot of `sequential` belongs to.
    first: usize,
}

impl<'r> Routes<'r> {
    /// The most slots `sequential` may have for each route it holds.
    const MAX_SLOTS_PER_ROUTE: usize = 4;

    /// Stores `routes` in `arena`.
    ///
    /// # Errors
    ///
    /// Returns a message if two routes have the same id.
    pub(crate) fn new(
        arena: &'r Bump,
        mut routes: Vec<RegisteredRoute<'r>>,
    ) -> Result<Self, String> {
        routes.sort_unstable_by_key(|route| route.id().number());
        if let Some(pair) = routes.windows(2).find(|pair| pair[0].id() == pair[1].id()) {
            return Err(format!(
                "routes `{}` and `{}` share an identity",
                pair[0].path(),
                pair[1].path()
            ));
        }
        let sorted: &'r [RegisteredRoute<'r>] = arena.alloc_slice_copy(&routes);
        let (Some(lowest), Some(highest)) = (sorted.first(), sorted.last()) else {
            return Ok(Self::default());
        };

        // The table has one slot per id from the lowest to the highest, so
        // it is only built when few of its slots would stay empty.
        let first = lowest.id().number();
        let slots = highest.id().number() - first + 1;
        let sequential: &'r [Option<&'r RegisteredRoute<'r>>] =
            if slots <= Self::MAX_SLOTS_PER_ROUTE * sorted.len() {
                let sequential = arena.alloc_slice_fill_copy(slots, None);
                for route in sorted {
                    sequential[route.id().number() - first] = Some(route);
                }
                sequential
            } else {
                &[]
            };

        Ok(Self {
            sorted,
            sequential,
            first,
        })
    }

    /// Returns the route registered under `id`, or `None` if there is none.
    pub(crate) fn get(&self, id: RouteId) -> Option<RegisteredRoute<'r>> {
        let number = id.number();
        let slot = number
            .checked_sub(self.first)
            .and_then(|position| self.sequential.get(position));
        if let Some(slot) = slot {
            return slot.copied();
        }
        let position = self
            .sorted
            .binary_search_by_key(&number, |route| route.id().number())
            .ok()?;
        Some(self.sorted[position])
    }

    /// Iterates over the routes in the order of their ids.
    pub(crate) fn iter(&self) -> impl Iterator<Item = RegisteredRoute<'r>> {
        self.sorted.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Method;

    fn unused(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
        unreachable!()
    }

    /// The route every registered route in these tests runs. The tests give
    /// each registered route its own id and path.
    fn handler() -> RouteFn {
        RouteFn::new(Method::GET, "/", unused)
    }

    fn entry(route: &RouteFn) -> RouteEntry<'_> {
        RouteEntry {
            handler: Handler::Route(route),
            layers: &[],
            context: None,
        }
    }

    /// A route for each id number in `numbers`, served at a path spelling
    /// out that number.
    fn registered<'r>(
        arena: &'r Bump,
        entry: &'r RouteEntry<'r>,
        numbers: &[usize],
    ) -> Vec<RegisteredRoute<'r>> {
        numbers
            .iter()
            .map(|&number| RegisteredRoute {
                id: RouteId(number),
                path: Path::new(arena.alloc_str(&format!("/{number}"))),
                entry,
            })
            .collect()
    }

    /// Stores a route for each id number in `numbers`.
    fn routes<'r>(arena: &'r Bump, entry: &'r RouteEntry<'r>, numbers: &[usize]) -> Routes<'r> {
        Routes::new(arena, registered(arena, entry, numbers)).unwrap()
    }

    /// The path of the route `routes` finds under the id numbered `number`.
    fn found<'r>(routes: &Routes<'r>, number: usize) -> Option<&'r str> {
        routes
            .get(RouteId(number))
            .map(|route| route.path().as_str())
    }

    #[test]
    fn ids_close_together_are_found_through_the_table() {
        let arena = Bump::new();
        let handler = handler();
        let entry = entry(&handler);
        let routes = routes(&arena, &entry, &[103, 100, 101]);
        assert!(!routes.sequential.is_empty());

        assert_eq!(found(&routes, 100), Some("/100"));
        assert_eq!(found(&routes, 101), Some("/101"));
        assert_eq!(found(&routes, 103), Some("/103"));
        // An id between the registered ones, and ids on either side of them.
        assert_eq!(found(&routes, 102), None);
        assert_eq!(found(&routes, 99), None);
        assert_eq!(found(&routes, 104), None);
    }

    #[test]
    fn ids_far_apart_are_found_without_a_table() {
        let arena = Bump::new();
        let handler = handler();
        let entry = entry(&handler);
        let routes = routes(&arena, &entry, &[90_000, 5, 5_000]);
        assert!(routes.sequential.is_empty());

        assert_eq!(found(&routes, 5), Some("/5"));
        assert_eq!(found(&routes, 5_000), Some("/5000"));
        assert_eq!(found(&routes, 90_000), Some("/90000"));
        assert_eq!(found(&routes, 6), None);
        assert_eq!(found(&routes, 0), None);
        assert_eq!(found(&routes, 90_001), None);
    }

    #[test]
    fn routes_are_listed_in_id_order() {
        let arena = Bump::new();
        let handler = handler();
        let entry = entry(&handler);
        let routes = routes(&arena, &entry, &[7, 3, 5]);

        let paths: Vec<&str> = routes.iter().map(|route| route.path().as_str()).collect();
        assert_eq!(paths, ["/3", "/5", "/7"]);
    }

    #[test]
    fn two_routes_with_one_id_are_rejected() {
        let arena = Bump::new();
        let handler = handler();
        let entry = entry(&handler);
        let routes = Routes::new(&arena, registered(&arena, &entry, &[4, 9, 4]));

        assert!(routes.is_err());
    }

    #[test]
    fn no_routes_find_nothing() {
        let arena = Bump::new();
        let handler = handler();
        let entry = entry(&handler);
        let routes = routes(&arena, &entry, &[]);

        assert_eq!(found(&routes, 0), None);
        assert_eq!(routes.iter().count(), 0);
    }
}
