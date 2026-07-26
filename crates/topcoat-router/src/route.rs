use std::{borrow::Cow, pin::Pin};

use topcoat_core::{context::Cx, error::Result};

use crate::{Body, Methods, OwnedMethods, Path, Response};

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
}

impl RouteFn {
    /// Creates a new route with explicit methods, path, and handler function.
    ///
    /// The methods are anything convertible into [`OwnedMethods`]: a single
    /// [`Method`](crate::Method), a `&'static [Method]`, a `Vec<Method>`, or
    /// [`Methods::Any`] to respond to every method.
    ///
    /// ```rust
    /// use std::borrow::Cow;
    ///
    /// use topcoat::context::Cx;
    /// use topcoat::router::{Body, Method, Path, RouteFn, RouteFuture};
    ///
    /// fn handler(_cx: &Cx, _body: Body) -> RouteFuture<'_> {
    ///     Box::pin(async move { unimplemented!() })
    /// }
    ///
    /// let form = RouteFn::new(
    ///     &[Method::GET, Method::POST],
    ///     Cow::Borrowed(Path::new("/form")),
    ///     handler,
    /// );
    /// ```
    pub fn new(
        methods: impl Into<OwnedMethods>,
        path: Cow<'static, Path>,
        handle: RouteHandlerFn,
    ) -> Self {
        Self::const_new(methods.into(), path, handle)
    }

    /// Const-context constructor used by macro-generated code.
    pub const fn const_new(
        methods: OwnedMethods,
        path: Cow<'static, Path>,
        handle: RouteHandlerFn,
    ) -> Self {
        Self {
            methods,
            path,
            handle,
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

    fn handle<'cx>(&'cx self, cx: &'cx Cx, body: Body) -> RouteFuture<'cx> {
        (self.handle)(cx, body)
    }
}

#[cfg(feature = "discover")]
inventory::collect!(RouteFn);
