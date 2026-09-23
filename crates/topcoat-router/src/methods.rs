use std::borrow::Cow;

use http::Method;

/// The HTTP methods a [`Route`](crate::Route) responds to, as returned by
/// [`Route::methods`](crate::Route::methods).
///
/// Most routes respond to a fixed list of methods, usually just one.
/// [`Methods::Any`] is for a route that accepts every method, like one that
/// forwards requests to another service. A route registered for a specific
/// method wins over an any-method route at the same path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Methods<'a> {
    /// The route accepts every HTTP method.
    Any,
    /// The route accepts only the listed methods.
    Only(&'a [Method]),
}

/// An owned version of [`Methods`], stored by routes like
/// [`RouteFn`](crate::RouteFn).
///
/// You rarely create one directly. Functions like
/// [`RouteFn::new`](crate::RouteFn::new) accept anything that converts into
/// it: a [`Method`], a `&'static [Method]`, a `Vec<Method>`, or a [`Methods`]
/// value such as [`Methods::Any`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnedMethods {
    /// Responds to one method.
    One(Method),
    /// Responds to each method in the set.
    Set(Cow<'static, [Method]>),
    /// Responds to every HTTP method.
    Any,
}

impl OwnedMethods {
    /// Returns these methods as a [`Methods`] value, as
    /// [`Route::methods`](crate::Route::methods) returns them.
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

#[cfg(test)]
mod tests {
    use super::*;

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
