use serde::de::DeserializeOwned;
use topcoat_core::context::Cx;

use crate::request::uri;

/// A struct that the request's query string is parsed into, declared with
/// [`#[query_params]`](attr.query_params.html).
///
/// The macro implements this trait. Do not implement it by hand. Read the
/// value with the [`query_params`] function.
pub trait QueryParams {
    /// The value [`query_params`] returns for this struct.
    ///
    /// It is a `Result` holding a reference to the parsed struct, or the
    /// error declared by the attribute. The default error is a reference to
    /// a [`QueryParamsError`].
    type Output<'cx>;

    /// Parses the query string of the request `cx` belongs to.
    ///
    /// This method cannot be called directly. Call
    /// [`query_params::<T>(cx)`](query_params) instead.
    #[doc(hidden)]
    fn query_params(cx: &Cx, _: QueryParamsSealed) -> Self::Output<'_>;
}

/// Parses the current request's query string into `T`.
///
/// `T` is a struct declared with
/// [`#[query_params]`](attr.query_params.html). See the macro for what this
/// returns.
#[inline]
#[must_use]
pub fn query_params<T: QueryParams>(cx: &Cx) -> T::Output<'_> {
    T::query_params(cx, QueryParamsSealed::new())
}

/// The error returned when the query string does not match the struct it is
/// parsed into.
///
/// It names the field that failed to parse.
pub type QueryParamsError = serde_path_to_error::Error<serde::de::value::Error>;

/// Parses the query string of the request `cx` belongs to into `T`.
///
/// Unlike [`query_params`], this parses the query string on every call and
/// works with any [`DeserializeOwned`] type.
///
/// # Errors
///
/// Returns a [`QueryParamsError`] when the query string does not deserialize
/// into `T`.
pub fn parse_query_params<T: DeserializeOwned>(cx: &Cx) -> Result<T, QueryParamsError> {
    let query = uri(cx).query().unwrap_or("");
    let deserializer =
        crate::urlencoded::Deserializer::new(form_urlencoded::parse(query.as_bytes()));
    serde_path_to_error::deserialize(deserializer)
}

/// A token that only this crate can create, so that
/// [`QueryParams::query_params`] can only be called through [`query_params`].
#[doc(hidden)]
#[derive(Debug)]
pub struct QueryParamsSealed(());

impl QueryParamsSealed {
    pub(crate) fn new() -> Self {
        Self(())
    }
}

#[cfg(test)]
mod tests {
    use http::Request;
    use topcoat_core::context::CxTestBuilder;

    use super::*;

    #[derive(Debug, serde::Deserialize)]
    struct Paging {
        page: Option<f64>,
    }

    /// Builds a `Cx` carrying request `Parts` for a `GET` of `uri`.
    fn cx(uri: &str) -> Cx {
        let (parts, ()) = Request::builder()
            .uri(uri)
            .body(())
            .expect("request should build")
            .into_parts();

        CxTestBuilder::new().request_context(parts).build()
    }

    #[test]
    fn parse_query_params_reads_empty_optional_value_as_none() {
        let paging: Paging =
            parse_query_params(&cx("/items?page=")).expect("an empty optional value");

        assert_eq!(paging.page, None);
    }

    #[test]
    fn parse_query_params_reads_present_optional_value() {
        let paging: Paging =
            parse_query_params(&cx("/items?page=2")).expect("a valid optional value");

        assert_eq!(paging.page, Some(2.0));
    }
}
