#[cfg(feature = "fs")]
mod directory;

use std::{
    borrow::Cow,
    collections::HashMap,
    num::NonZeroUsize,
    ops::Index,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

#[cfg(feature = "fs")]
pub use directory::*;
use topcoat_core::{context::Cx, error::Result};

use crate::{
    Body, EndpointIndex, HrefTarget, IntoPath, Layer, Methods, Next, OwnedMethods, Path, Terminal,
    response::Response, route, route_endpoint,
};

/// The future returned by [`Route::handle`].
///
/// It is boxed and `Send`, and it may borrow the route and the request
/// context.
pub type RouteFuture<'cx> = Pin<Box<dyn Future<Output = Result<Response>> + Send + 'cx>>;

/// A unique id for a route handler.
///
/// Topcoat uses it to tell routes apart, for example in
/// [`Route::is_current`]. Each handler creates one id with
/// [`new`](RouteId::new) and returns it from its `id` method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RouteId(usize);

impl RouteId {
    /// Creates an id that differs from every other id in the process.
    #[must_use]
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}

/// A request handler for a URL path and a set of HTTP methods.
///
/// Routes are what a [`Router`](crate::Router) runs to answer requests.
/// Pages, and handlers declared with `#[route]`, become routes. Register a
/// route with [`RouterBuilder::route`](crate::RouterBuilder::route), or use
/// [`RouteFn`] to make one from a function.
pub trait Route: Send + Sync + 'static {
    /// Returns the id of this route's handler.
    ///
    /// Return the same [`RouteId`] on every call.
    fn id(&self) -> RouteId;

    /// Returns the HTTP methods this route responds to.
    fn methods(&self) -> Methods<'_>;

    /// Returns the path pattern this route handles.
    fn path(&self) -> &Path;

    /// Handles a request and produces a response.
    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx>;

    /// Returns whether this route is the one handling the current request.
    ///
    /// Only the route's [`id`](Self::id) is compared, so the result does not
    /// depend on the values of path parameters or on the query string.
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

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        (**self).handle(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(&'static dyn Route);

/// The handler function of a [`RouteFn`].
pub type RouteHandlerFn = for<'cx> fn(cx: &'cx Cx, body: Body) -> RouteFuture<'cx>;

/// A [`Route`] made from a handler function, a path, and a set of methods.
///
/// Use it to register a route without implementing [`Route`] on a type of
/// your own.
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
}

impl RouteFn {
    /// Creates a route that runs `handle` for requests to `path` with one of
    /// `methods`.
    ///
    /// `methods` accepts anything that converts into [`OwnedMethods`], like a
    /// single [`Method`](crate::Method), a slice or `Vec` of methods, or
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
        }
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

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        (self.handle)(cx, body)
    }
}

impl HrefTarget for RouteFn {
    #[track_caller]
    fn path<'cx>(&self, cx: &'cx Cx) -> &'cx Path {
        match route_endpoint(cx, self.id) {
            Some(endpoint) => endpoint.path(),
            None => panic!(
                "route `{}` is not registered on the router serving this request",
                self.path
            ),
        }
    }
}

/// The position of a route in a router's [`Routes`] table.
///
/// Stored offset by one in a [`NonZeroUsize`] so that `Option<RouteIndex>`
/// occupies a single word, keeping an endpoint's per-method table dense.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RouteIndex(NonZeroUsize);

impl RouteIndex {
    /// Wraps a route's position in the table.
    pub(crate) fn new(index: usize) -> Self {
        Self(NonZeroUsize::new(index.wrapping_add(1)).expect("route index overflow"))
    }

    /// Returns the wrapped position.
    pub(crate) fn get(self) -> usize {
        self.0.get() - 1
    }
}

/// A route paired with the layers that wrap it.
pub(crate) struct RouteWithLayers {
    /// The route itself.
    route: Box<dyn Route>,
    /// The layers wrapping this route, precomputed at build time from the
    /// route's path (group segments included) and ordered from least- to
    /// most-specific so the outermost layer runs first.
    layers: Box<[Arc<dyn Layer>]>,
}

impl Route for RouteWithLayers {
    fn id(&self) -> RouteId {
        self.route.id()
    }

    fn methods(&self) -> Methods<'_> {
        self.route.methods()
    }

    fn path(&self) -> &Path {
        self.route.path()
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        Next::new(&self.layers, Terminal::Route(&*self.route)).run(cx, body)
    }
}

/// The routes registered on a router, in registration order, indexed by
/// [`RouteIndex`].
///
/// Routes are [`push`](Self::push)ed as the router is built, then only
/// queried: [`endpoint`](Self::endpoint) resolves a route's [`RouteId`] to the
/// endpoint serving it, and indexing by [`RouteIndex`] resolves a position
/// back to the route and its layers.
#[derive(Default)]
pub(crate) struct Routes {
    routes: Vec<RouteWithLayers>,
    endpoint_lookup: HashMap<RouteId, EndpointIndex>,
}

impl Routes {
    /// Registers `route` as served by `endpoint` and wrapped by `layers`,
    /// returning the [`RouteIndex`] that now identifies the registration.
    pub(crate) fn push(
        &mut self,
        route: Box<dyn Route>,
        endpoint: EndpointIndex,
        layers: Box<[Arc<dyn Layer>]>,
    ) -> RouteIndex {
        let index = RouteIndex::new(self.routes.len());
        self.endpoint_lookup.insert(route.id(), endpoint);
        self.routes.push(RouteWithLayers { route, layers });
        index
    }

    /// Returns the endpoint serving the route registered under `id`, or `None` if
    /// this router holds no route with that identity.
    pub(crate) fn endpoint(&self, id: RouteId) -> Option<EndpointIndex> {
        self.endpoint_lookup.get(&id).copied()
    }
}

impl Index<RouteIndex> for Routes {
    type Output = RouteWithLayers;

    fn index(&self, index: RouteIndex) -> &Self::Output {
        &self.routes[index.get()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- RouteIndex --

    #[test]
    fn route_index_wraps_and_unwraps() {
        let index = RouteIndex::new(7);
        assert_eq!(index.get(), 7);
    }

    #[test]
    fn route_index_zero_is_a_real_index() {
        // The offset keeps index 0 representable despite the non-zero backing.
        let index = RouteIndex::new(0);
        assert_eq!(index.get(), 0);
    }

    #[test]
    fn option_route_index_stays_one_word() {
        assert_eq!(
            std::mem::size_of::<Option<RouteIndex>>(),
            std::mem::size_of::<usize>()
        );
    }
}
