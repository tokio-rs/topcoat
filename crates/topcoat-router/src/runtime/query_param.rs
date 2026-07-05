use topcoat_core::runtime::context::Cx;

/// A typed view of the request's query string, as declared by
/// [`#[query_params]`](attr.query_params.html).
///
/// This trait is implemented by the macro and is not meant to be implemented by
/// hand. Read the value with the [`query_params`] free function rather than
/// calling the trait method directly.
pub trait QueryParams: Sized {
    /// Parses the query string of the request `cx` belongs to.
    ///
    /// Call [`query_params::<T>(cx)`](query_params) instead: this method is
    /// sealed behind [`QueryParamsSealed`] and cannot be invoked directly.
    #[doc(hidden)]
    fn query_params(cx: &Cx, _: QueryParamsSealed) -> Result<&Self, &serde_urlencoded::de::Error>;
}

/// Parses the request's query string into a typed struct.
///
/// See [`#[query_params]`](attr.query_params.html) for details.
///
/// # Errors
///
/// Returns a reference to the deserialization error if the query string cannot
/// be parsed into `T`.
#[inline]
pub fn query_params<T: QueryParams>(cx: &Cx) -> Result<&T, &serde_urlencoded::de::Error> {
    T::query_params(cx, QueryParamsSealed::new())
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
