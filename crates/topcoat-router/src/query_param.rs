use serde::de::DeserializeOwned;
use topcoat_core::context::Cx;

use crate::context::uri;

/// A typed view of the request's query string, as declared by
/// [`#[query_params]`](attr.query_params.html).
///
/// This trait is implemented by the macro and is not meant to be implemented by
/// hand. Read the value with the [`query_params`] free function rather than
/// calling the trait method directly.
pub trait QueryParams {
    /// The value produced for a request bound to lifetime `'cx`.
    ///
    /// A `Result` of a reference to the parsed struct or the error declared
    /// by the attribute (a reference to the [`QueryParamsError`] by default).
    type Output<'cx>;

    /// Parses the query string of the request `cx` belongs to.
    ///
    /// Call [`query_params::<T>(cx)`](query_params) instead: this method is
    /// sealed behind [`QueryParamsSealed`] and cannot be invoked directly.
    #[doc(hidden)]
    fn query_params(cx: &Cx, _: QueryParamsSealed) -> Self::Output<'_>;
}

/// Parses the request's query string into a typed struct.
///
/// See [`#[query_params]`](attr.query_params.html) for details.
#[inline]
#[must_use]
pub fn query_params<T: QueryParams>(cx: &Cx) -> T::Output<'_> {
    T::query_params(cx, QueryParamsSealed::new())
}

/// The error produced when the request's query string fails to deserialize,
/// carrying the path of the key that failed.
pub type QueryParamsError = serde_path_to_error::Error<serde_urlencoded::de::Error>;

/// Deserializes the query string of the request `cx` belongs to into `T`.
///
/// This backs the typed `#[query_params]` accessors and is rarely used
/// directly.
///
/// # Errors
///
/// Returns a [`QueryParamsError`] when the query string does not deserialize
/// into `T`.
pub fn parse_query_params<T: DeserializeOwned>(cx: &Cx) -> Result<T, QueryParamsError> {
    let query = uri(cx).query().unwrap_or("");
    let deserializer =
        serde_urlencoded::Deserializer::new(form_urlencoded::parse(query.as_bytes()));
    serde_path_to_error::deserialize(deserializer)
}

/// A guard that limits [`QueryParams::query_params`] to being called through the
/// [`query_params`] free function.
///
/// It cannot be constructed outside this crate, so the only way to invoke the
/// trait method is via [`query_params`].
#[doc(hidden)]
#[derive(Debug)]
pub struct QueryParamsSealed(());

impl QueryParamsSealed {
    pub(crate) fn new() -> Self {
        Self(())
    }
}
