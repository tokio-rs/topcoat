use std::{borrow::Cow, panic::Location, pin::Pin};

use topcoat_core::{context::Cx, error::Result};

use crate::{Body, IntoPath, Methods, OwnedMethods, Path, response::Response};

/// The future returned by [`Route::handle`]: a boxed, `Send` future borrowing
/// the route and its request context.
pub type RouteFuture<'cx> = Pin<Box<dyn Future<Output = Result<Response>> + Send + 'cx>>;

/// A single routable endpoint: a set of HTTP methods, a URL path, and a
/// handler.
///
/// This is the core primitive a [`Router`](crate::Router) dispatches to.
/// Register any `Route` with [`RouterBuilder::route`](crate::RouterBuilder::route).
pub trait Route: Send + Sync + 'static {
    /// The HTTP methods this route responds to.
    fn methods(&self) -> Methods<'_>;

    /// The URL path this route handles.
    fn path(&self) -> &Path;

    /// Returns where this route was declared or registered.
    #[doc(hidden)]
    fn source_location(&self) -> Option<&'static Location<'static>> {
        None
    }

    /// Handles a request, producing a response.
    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx>;
}

/// The async handler function backing a [`RouteFn`].
pub type RouteHandlerFn = for<'cx> fn(cx: &'cx Cx, body: Body) -> RouteFuture<'cx>;

/// A [`Route`] backed by a plain handler function.
///
/// Created either manually via `#[route(GET "/path")]` or by the module router
/// (which derives the path from the module tree). Registered into a
/// [`Router`](crate::Router).
#[derive(Debug, Clone)]
pub struct RouteFn {
    /// The HTTP methods this route responds to.
    methods: OwnedMethods,
    /// The URL path this route handles.
    path: Cow<'static, Path>,
    /// The handler function that produces the response.
    handle: RouteHandlerFn,
    /// Where the route was declared.
    source_location: &'static Location<'static>,
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
        Self::const_new(methods.into(), path.into_path(), handle)
    }

    /// Const-context constructor used by macro-generated code.
    #[track_caller]
    pub const fn const_new(
        methods: OwnedMethods,
        path: Cow<'static, Path>,
        handle: RouteHandlerFn,
    ) -> Self {
        Self::with_source_location(methods, path, handle, Location::caller())
    }

    /// Const-context constructor that preserves an earlier declaration site.
    pub(crate) const fn with_source_location(
        methods: OwnedMethods,
        path: Cow<'static, Path>,
        handle: RouteHandlerFn,
        source_location: &'static Location<'static>,
    ) -> Self {
        Self {
            methods,
            path,
            handle,
            source_location,
        }
    }
}

impl Route for RouteFn {
    fn methods(&self) -> Methods<'_> {
        self.methods.as_methods()
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn source_location(&self) -> Option<&'static Location<'static>> {
        Some(self.source_location)
    }

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        (self.handle)(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(RouteFn);
