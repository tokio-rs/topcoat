use std::{borrow::Cow, pin::Pin};

use http::Method;
use topcoat_core::{context::Cx, error::Result};

use crate::{Body, Path, Response};

/// The future returned by [`Route::handle`]: a boxed, `Send` future borrowing
/// the route and its request context.
pub type RouteFuture<'cx> = Pin<Box<dyn Future<Output = Result<Response>> + Send + 'cx>>;

/// The HTTP methods a [`Route`] responds to, as returned by
/// [`Route::methods`].
///
/// Most routes respond to a fixed set of methods, usually a single one.
/// [`Methods::Any`] marks a route that accepts every method at its path, like
/// an adapter forwarding requests to an external service. A route registered
/// for a specific method takes precedence over an any-method route at the
/// same path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Methods<'a> {
    /// The route accepts every HTTP method.
    Any,
    /// The route accepts exactly the listed methods.
    Only(&'a [Method]),
}

/// An owned counterpart to [`Methods`], stored by routes like [`RouteFn`].
///
/// Rarely constructed directly: [`RouteFn::new`] accepts anything convertible
/// into it, like a [`Method`], a `&'static [Method]`, a `Vec<Method>`, or a
/// [`Methods`] value (so [`Methods::Any`] expresses an any-method route). The
/// common single-method case is stored without allocating.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedMethods {
    /// Responds to a single method.
    One(Method),
    /// Responds to each method in the set.
    Set(Cow<'static, [Method]>),
    /// Responds to every HTTP method.
    Any,
}

impl OwnedMethods {
    /// Borrows this set as a [`Methods`] value, as returned by
    /// [`Route::methods`].
    #[must_use]
    pub fn as_methods(&self) -> Methods<'_> {
        match self {
            Self::One(method) => Methods::Only(std::slice::from_ref(method)),
            Self::Set(methods) => Methods::Only(methods),
            Self::Any => Methods::Any,
        }
    }
}

impl From<Method> for OwnedMethods {
    fn from(method: Method) -> Self {
        Self::One(method)
    }
}

impl From<&'static [Method]> for OwnedMethods {
    fn from(methods: &'static [Method]) -> Self {
        Self::Set(Cow::Borrowed(methods))
    }
}

impl<const N: usize> From<&'static [Method; N]> for OwnedMethods {
    fn from(methods: &'static [Method; N]) -> Self {
        Self::Set(Cow::Borrowed(methods))
    }
}

impl From<Vec<Method>> for OwnedMethods {
    fn from(methods: Vec<Method>) -> Self {
        Self::Set(Cow::Owned(methods))
    }
}

impl From<Cow<'static, [Method]>> for OwnedMethods {
    fn from(methods: Cow<'static, [Method]>) -> Self {
        Self::Set(methods)
    }
}

impl From<Methods<'static>> for OwnedMethods {
    fn from(methods: Methods<'static>) -> Self {
        match methods {
            Methods::Any => Self::Any,
            Methods::Only(methods) => Self::Set(Cow::Borrowed(methods)),
        }
    }
}

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
    /// [`Method`], a `&'static [Method]`, a `Vec<Method>`, or [`Methods::Any`]
    /// to respond to every method.
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

#[cfg(test)]
mod tests {
    use super::*;

    // -- OwnedMethods --

    #[test]
    fn a_single_method_converts_without_allocating() {
        let methods = OwnedMethods::from(Method::GET);
        assert_eq!(methods, OwnedMethods::One(Method::GET));
        assert_eq!(methods.as_methods(), Methods::Only(&[Method::GET]));
    }

    #[test]
    fn slices_arrays_and_vectors_convert_to_sets() {
        let expected = Methods::Only(&[Method::GET, Method::POST][..]);

        let slice: &'static [Method] = &[Method::GET, Method::POST];
        assert_eq!(OwnedMethods::from(slice).as_methods(), expected);
        assert_eq!(
            OwnedMethods::from(&[Method::GET, Method::POST]).as_methods(),
            expected
        );
        assert_eq!(
            OwnedMethods::from(vec![Method::GET, Method::POST]).as_methods(),
            expected
        );
    }

    #[test]
    fn methods_values_convert_losslessly() {
        assert_eq!(OwnedMethods::from(Methods::Any), OwnedMethods::Any);
        assert_eq!(OwnedMethods::Any.as_methods(), Methods::Any);
        assert_eq!(
            OwnedMethods::from(Methods::Only(&[Method::PUT])).as_methods(),
            Methods::Only(&[Method::PUT])
        );
    }
}
