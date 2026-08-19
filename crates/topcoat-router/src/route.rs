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

use topcoat_core::{context::Cx, error::Result};

use crate::{
    Body, EndpointIndex, HrefTarget, IntoPath, Layer, Methods, OwnedMethods, Path,
    response::Response, route, route_endpoint,
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

/// A route registered on a router, tied to the endpoint serving its path.
pub(crate) struct RegisteredRoute {
    /// The route itself.
    pub(crate) route: Box<dyn Route>,
    /// The endpoint the route's path resolved to, where its URL path lives.
    pub(crate) endpoint: EndpointIndex,
    /// The layers wrapping this route, precomputed at build time from the
    /// route's path (group segments included) and ordered from least- to
    /// most-specific so the outermost layer runs first.
    pub(crate) layers: Box<[Arc<dyn Layer>]>,
}

/// The routes registered on a router, in registration order, indexed by
/// [`RouteIndex`].
///
/// Routes are [`push`](Self::push)ed as the router is built, then only
/// queried: [`index_of`](Self::index_of) resolves a route's [`RouteId`] to its
/// position, and indexing by [`RouteIndex`] resolves a position back to the
/// registration.
#[derive(Default)]
pub(crate) struct Routes {
    routes: Vec<RegisteredRoute>,
    by_id: HashMap<RouteId, RouteIndex>,
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
        self.by_id.insert(route.id(), index);
        self.routes.push(RegisteredRoute {
            route,
            endpoint,
            layers,
        });
        index
    }

    /// Returns the position of the route registered under `id`, or `None` if
    /// this router holds no route with that identity.
    pub(crate) fn index_of(&self, id: RouteId) -> Option<RouteIndex> {
        self.by_id.get(&id).copied()
    }
}

impl Index<RouteIndex> for Routes {
    type Output = RegisteredRoute;

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
