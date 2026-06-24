use std::sync::Arc;

use percent_encoding::percent_decode_str;
use topcoat_core::runtime::context::Cx;

/// A typed view of a single path parameter, as declared by
/// [`#[path_param]`](attr.path_param.html).
///
/// This trait is implemented by the macro and is not meant to be implemented by
/// hand. Read the value with the [`path_param`] free function rather than
/// calling the trait method directly.
pub trait PathParam {
    /// The value produced for a request bound to lifetime `'cx`.
    ///
    /// For a `&str` inner type this is the param struct itself (borrowing the
    /// segment); for any other inner type it is a `Result` of a reference to the
    /// parsed struct or the [`FromStr`](core::str::FromStr) error.
    type Output<'cx>;

    /// Reads the parameter from the request `cx` belongs to.
    ///
    /// Call [`path_param::<T>(cx)`](path_param) instead — this method is sealed
    /// behind [`PathParamSealed`] and cannot be invoked directly.
    #[doc(hidden)]
    fn path_param(cx: &Cx, _: PathParamSealed) -> Self::Output<'_>;
}

/// Reads a typed path parameter from the matched route's path.
///
/// See [`#[path_param]`](attr.path_param.html) for details.
#[inline]
#[must_use]
pub fn path_param<T: PathParam>(cx: &Cx) -> T::Output<'_> {
    T::path_param(cx, PathParamSealed::new())
}

/// The path parameters captured from the matched route.
///
/// Iterating over a reference yields each `(name, value)` pair as string
/// slices, with values percent-decoded. This backs the typed `#[path_param]`
/// accessors and is rarely used directly.
#[derive(Debug, Clone, Default)]
pub struct RawPathParams(Vec<(Arc<str>, Box<str>)>);

impl RawPathParams {
    /// Captures the parameters of a route match, percent-decoding each value.
    pub(crate) fn from_pairs<'pairs>(
        pairs: impl IntoIterator<Item = (&'pairs str, &'pairs str)>,
    ) -> Self {
        Self(
            pairs
                .into_iter()
                .map(|(key, value)| {
                    let value = percent_decode_str(value)
                        .decode_utf8_lossy()
                        .into_owned()
                        .into_boxed_str();
                    (Arc::from(key), value)
                })
                .collect(),
        )
    }

    /// Iterates over each `(name, value)` pair as string slices.
    pub fn iter(&self) -> RawPathParamsIter<'_> {
        <&Self as IntoIterator>::into_iter(self)
    }
}

pub type RawPathParamsIter<'params> = std::iter::Map<
    std::slice::Iter<'params, (Arc<str>, Box<str>)>,
    fn(&'params (Arc<str>, Box<str>)) -> (&'params str, &'params str),
>;

impl<'params> IntoIterator for &'params RawPathParams {
    type Item = (&'params str, &'params str);
    type IntoIter = RawPathParamsIter<'params>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().map(|(key, value)| (&**key, &**value))
    }
}

/// A guard that limits [`PathParam::path_param`] to being called through the
/// [`path_param`] free function.
///
/// It cannot be constructed outside this crate, so the only way to invoke the
/// trait method is via [`path_param`].
#[doc(hidden)]
#[derive(Debug)]
pub struct PathParamSealed(());

impl PathParamSealed {
    pub(crate) fn new() -> Self {
        Self(())
    }
}
