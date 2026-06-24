use std::ops::{Deref, DerefMut};

use ::serde::{Serialize, de::DeserializeOwned};
use http::{
    Method,
    header::{CONTENT_TYPE, HeaderValue},
};
use topcoat_core::runtime::{context::Cx, error::Result};

use crate::runtime::{
    Body, Bytes, FromRequest, IntoResponse, OptionalFromRequest, Response, bad_request,
    bad_request_at, content_type, method, to_bytes, uri,
};

/// `application/x-www-form-urlencoded` request extractor and response wrapper.
///
/// As a [`FromRequest`] extractor, `Form<T>` deserializes URL-encoded form data
/// into `T`. For `GET` and `HEAD` requests it reads the URI query string; for
/// other methods it reads the request body and requires a
/// `Content-Type: application/x-www-form-urlencoded` header. As an
/// [`IntoResponse`] wrapper, it serializes `T` back to a URL-encoded body and
/// sets the response `Content-Type`.
///
/// Wrap it in [`Option`] to make the body optional. For `GET` and `HEAD`
/// requests the extractor yields [`None`] only when there is no query string;
/// for other methods it yields [`None`] when there is no `Content-Type` header.
///
/// # Examples
///
/// ```rust
/// use serde::{Deserialize, Serialize};
/// use topcoat::{
///     Result,
///     router::{Form, Json, route},
/// };
///
/// #[derive(Deserialize, Serialize)]
/// struct Search {
///     query: String,
///     page: u32,
/// }
///
/// #[route(GET "/search")]
/// async fn search(Form(input): Form<Search>) -> Result<Json<Search>> {
///     Ok(Json(input))
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
#[must_use]
pub struct Form<T>(pub T);

impl<T> From<T> for Form<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> Deref for Form<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Form<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> FromRequest for Form<T>
where
    T: DeserializeOwned,
{
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        let RawForm(bytes) = RawForm::from_request(cx, body).await?;
        Self::from_bytes(&bytes)
    }
}

impl<T> OptionalFromRequest for Form<T>
where
    T: DeserializeOwned,
{
    async fn from_request(cx: &Cx, body: Body) -> Result<Option<Self>> {
        if matches!(method(cx), &Method::GET | &Method::HEAD) {
            return match uri(cx).query() {
                Some(query) => Ok(Some(Self::from_bytes(query.as_bytes())?)),
                None => Ok(None),
            };
        }

        if content_type(cx).is_some() {
            Ok(Some(<Self as FromRequest>::from_request(cx, body).await?))
        } else {
            Ok(None)
        }
    }
}

impl<T> Form<T>
where
    T: DeserializeOwned,
{
    /// Deserializes URL-encoded form bytes into `Form<T>`.
    ///
    /// Unlike the [`FromRequest`] extractor, this does not inspect the request
    /// method or `Content-Type`; it parses `bytes` directly.
    ///
    /// # Errors
    ///
    /// Returns a bad-request error when `bytes` are not valid URL-encoded form
    /// data matching `T`.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let deserializer = serde_urlencoded::Deserializer::new(form_urlencoded::parse(bytes));
        let value = serde_path_to_error::deserialize(deserializer).map_err(|error| {
            bad_request_at(
                error.path(),
                format!("invalid form value: {}", error.inner()),
            )
        })?;

        Ok(Self(value))
    }
}

impl<T> IntoResponse for Form<T>
where
    T: Serialize,
{
    fn into_response(self) -> Result<Response> {
        (
            [(
                CONTENT_TYPE,
                HeaderValue::from_static("application/x-www-form-urlencoded"),
            )],
            serde_urlencoded::to_string(&self.0)?,
        )
            .into_response()
    }
}

/// Extractor for the raw bytes of an `application/x-www-form-urlencoded`
/// request.
///
/// For `GET` and `HEAD` requests it yields the raw query string; for other
/// methods it yields the raw request body and requires a
/// `Content-Type: application/x-www-form-urlencoded` header. Unlike [`Form`],
/// the bytes are returned without deserialization.
#[derive(Debug, Clone, Default)]
#[must_use]
pub struct RawForm(pub Bytes);

impl FromRequest for RawForm {
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        if matches!(method(cx), &Method::GET | &Method::HEAD) {
            let query = uri(cx).query().unwrap_or_default();
            return Ok(Self(Bytes::copy_from_slice(query.as_bytes())));
        }

        if !form_content_type(content_type(cx)) {
            return Err(bad_request(
                "expected request with `Content-Type: application/x-www-form-urlencoded`",
            )
            .into());
        }

        let bytes = to_bytes(body, usize::MAX)
            .await
            .map_err(|error| bad_request(format!("failed to read request body: {error}")))?;

        Ok(Self(bytes))
    }
}

/// Returns whether `content_type` is `application/x-www-form-urlencoded`,
/// ignoring any media type parameters (such as `; charset=utf-8`) and case.
fn form_content_type(content_type: Option<&str>) -> bool {
    let Some(content_type) = content_type else {
        return false;
    };

    content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .eq_ignore_ascii_case("application/x-www-form-urlencoded")
}

#[cfg(test)]
mod tests {
    use http::{Method, Request, header::CONTENT_TYPE};
    use topcoat_core::runtime::context::Cx;

    use super::*;
    use crate::runtime::{BadRequestError, Body, FromRequest, OptionalFromRequest, to_bytes};

    const FORM_CONTENT_TYPE: &str = "application/x-www-form-urlencoded";

    /// Builds a `Cx` carrying request `Parts` with the given method, URI, and
    /// optional `Content-Type` header.
    fn cx(method: Method, uri: &str, content_type: Option<&str>) -> Cx {
        let mut builder = Request::builder().method(method).uri(uri);
        if let Some(content_type) = content_type {
            builder = builder.header(CONTENT_TYPE, content_type);
        }

        let (parts, ()) = builder.body(()).expect("request should build").into_parts();

        let mut cx = Cx::empty();
        cx.insert(parts);
        cx
    }

    #[tokio::test]
    async fn from_request_reads_query_for_get() {
        let cx = cx(Method::GET, "/search?a=1&b=two", None);
        let Form(pairs) =
            <Form<Vec<(String, String)>> as FromRequest>::from_request(&cx, Body::empty())
                .await
                .expect("a valid query string");

        assert_eq!(
            pairs,
            vec![
                ("a".to_owned(), "1".to_owned()),
                ("b".to_owned(), "two".to_owned())
            ]
        );
    }

    #[tokio::test]
    async fn from_request_reads_body_for_post() {
        let cx = cx(Method::POST, "/search", Some(FORM_CONTENT_TYPE));
        let Form(pairs) = <Form<Vec<(String, String)>> as FromRequest>::from_request(
            &cx,
            Body::from("a=1&b=two"),
        )
        .await
        .expect("a valid form body");

        assert_eq!(
            pairs,
            vec![
                ("a".to_owned(), "1".to_owned()),
                ("b".to_owned(), "two".to_owned())
            ]
        );
    }

    #[tokio::test]
    async fn from_request_post_without_content_type_is_bad_request() {
        let cx = cx(Method::POST, "/search", None);
        let error =
            <Form<Vec<(String, String)>> as FromRequest>::from_request(&cx, Body::from("a=1"))
                .await
                .expect_err("a missing form content type is rejected");

        assert!(error.downcast_ref::<BadRequestError>().is_some());
    }

    #[tokio::test]
    async fn optional_from_request_get_without_query_is_none() {
        let cx = cx(Method::GET, "/search", None);
        let form =
            <Form<Vec<(String, String)>> as OptionalFromRequest>::from_request(&cx, Body::empty())
                .await
                .expect("an absent query string is not an error");

        assert!(form.is_none());
    }

    #[tokio::test]
    async fn optional_from_request_get_with_query_is_some() {
        let cx = cx(Method::GET, "/search?a=1", None);
        let form =
            <Form<Vec<(String, String)>> as OptionalFromRequest>::from_request(&cx, Body::empty())
                .await
                .expect("a valid query string is not an error");

        assert_eq!(
            form.expect("a form payload is present").0,
            vec![("a".to_owned(), "1".to_owned())]
        );
    }

    #[tokio::test]
    async fn optional_from_request_post_without_content_type_is_none() {
        let cx = cx(Method::POST, "/search", None);
        let form = <Form<Vec<(String, String)>> as OptionalFromRequest>::from_request(
            &cx,
            Body::from("a=1"),
        )
        .await
        .expect("an absent content type is not an error");

        assert!(form.is_none());
    }

    #[tokio::test]
    async fn optional_from_request_post_with_content_type_is_some() {
        let cx = cx(Method::POST, "/search", Some(FORM_CONTENT_TYPE));
        let form = <Form<Vec<(String, String)>> as OptionalFromRequest>::from_request(
            &cx,
            Body::from("a=1"),
        )
        .await
        .expect("a valid form body is not an error");

        assert_eq!(
            form.expect("a form payload is present").0,
            vec![("a".to_owned(), "1".to_owned())]
        );
    }

    #[test]
    fn from_bytes_deserializes_into_target_type() {
        let Form(pairs) =
            Form::<Vec<(String, u32)>>::from_bytes(b"a=1&b=2").expect("valid form data");
        assert_eq!(pairs, vec![("a".to_owned(), 1), ("b".to_owned(), 2)]);
    }

    #[test]
    fn from_bytes_rejects_values_that_do_not_match_the_target() {
        let error = Form::<Vec<(String, u32)>>::from_bytes(b"a=not-a-number")
            .expect_err("a type mismatch is rejected");

        assert!(error.downcast_ref::<BadRequestError>().is_some());
    }

    #[tokio::test]
    async fn into_response_serializes_form_with_content_type() {
        let response = Form(vec![
            ("a".to_owned(), "1".to_owned()),
            ("b".to_owned(), "two".to_owned()),
        ])
        .into_response()
        .expect("serialization succeeds");

        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .map(http::HeaderValue::as_bytes),
            Some(FORM_CONTENT_TYPE.as_bytes())
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("reading the response body");
        assert_eq!(&body[..], b"a=1&b=two");
    }

    #[tokio::test]
    async fn raw_form_get_yields_query_bytes() {
        let cx = cx(Method::GET, "/search?a=1&b=two", None);
        let RawForm(bytes) = RawForm::from_request(&cx, Body::empty())
            .await
            .expect("a query string");

        assert_eq!(&bytes[..], b"a=1&b=two");
    }

    #[tokio::test]
    async fn raw_form_post_yields_body_bytes() {
        let cx = cx(Method::POST, "/search", Some(FORM_CONTENT_TYPE));
        let RawForm(bytes) = RawForm::from_request(&cx, Body::from("a=1&b=two"))
            .await
            .expect("a form body");

        assert_eq!(&bytes[..], b"a=1&b=two");
    }

    #[tokio::test]
    async fn raw_form_post_without_content_type_is_bad_request() {
        let cx = cx(Method::POST, "/search", None);
        let error = RawForm::from_request(&cx, Body::from("a=1"))
            .await
            .expect_err("a missing form content type is rejected");

        assert!(error.downcast_ref::<BadRequestError>().is_some());
    }

    #[test]
    fn form_content_type_recognizes_urlencoded_media_types() {
        assert!(form_content_type(Some(FORM_CONTENT_TYPE)));
        assert!(form_content_type(Some(
            "application/x-www-form-urlencoded; charset=utf-8"
        )));
        assert!(form_content_type(Some("APPLICATION/X-WWW-FORM-URLENCODED")));

        assert!(!form_content_type(None));
        assert!(!form_content_type(Some("application/json")));
        assert!(!form_content_type(Some("text/plain")));
    }
}
