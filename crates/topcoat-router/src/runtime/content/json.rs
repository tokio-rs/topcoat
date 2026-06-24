use std::ops::{Deref, DerefMut};

use ::serde::{Serialize, de::DeserializeOwned};
use http::header::{CONTENT_TYPE, HeaderValue};
use topcoat_core::runtime::{
    context::Cx,
    error::{Error, Result},
};

use crate::runtime::{
    Body, FromRequest, IntoResponse, OptionalFromRequest, Response, bad_request, bad_request_at,
    content_type, to_bytes,
};

/// JSON request extractor and response wrapper.
///
/// As a [`FromRequest`] extractor, `Json<T>` reads the request body and
/// deserializes it into `T`. The request must carry a
/// `Content-Type: application/json` header, or any `application/*+json` media
/// type. As an [`IntoResponse`] wrapper, it serializes `T` to JSON and sets the
/// response `Content-Type` to `application/json`.
///
/// Wrap it in [`Option`] to make the body optional: the extractor yields
/// [`None`] when the request has no `Content-Type` header, and still reports an
/// error when a body is present but malformed.
///
/// # Examples
///
/// ```rust
/// use serde::{Deserialize, Serialize};
/// use topcoat::{
///     Result,
///     router::{Json, route},
/// };
///
/// #[derive(Deserialize)]
/// struct CreateUser {
///     name: String,
/// }
///
/// #[derive(Serialize)]
/// struct User {
///     id: u64,
///     name: String,
/// }
///
/// #[route(POST "/api/users")]
/// async fn create_user(Json(input): Json<CreateUser>) -> Result<Json<User>> {
///     Ok(Json(User {
///         id: 1,
///         name: input.name,
///     }))
/// }
/// ```
#[derive(Debug, Clone, Copy, Default)]
#[must_use]
pub struct Json<T>(pub T);

impl<T> From<T> for Json<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T> FromRequest for Json<T>
where
    T: DeserializeOwned,
{
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        if !json_content_type(content_type(cx)) {
            return Err(
                bad_request("expected request with `Content-Type: application/json`").into(),
            );
        }

        let bytes = to_bytes(body, usize::MAX)
            .await
            .map_err(|error| bad_request(format!("failed to read request body: {error}")))?;

        Self::from_bytes(&bytes)
    }
}

impl<T> OptionalFromRequest for Json<T>
where
    T: DeserializeOwned,
{
    async fn from_request(cx: &Cx, body: Body) -> Result<Option<Self>> {
        if content_type(cx).is_some() {
            Ok(Some(<Self as FromRequest>::from_request(cx, body).await?))
        } else {
            Ok(None)
        }
    }
}

impl<T> Json<T>
where
    T: DeserializeOwned,
{
    /// Deserializes JSON bytes into `Json<T>`.
    ///
    /// Unlike the [`FromRequest`] extractor, this does not inspect any
    /// `Content-Type`; it parses `bytes` directly.
    ///
    /// # Errors
    ///
    /// Returns a bad-request error when `bytes` are not valid JSON or do not
    /// match `T`, or when trailing data follows the JSON value.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let mut deserializer = serde_json::Deserializer::from_slice(bytes);
        let value = serde_path_to_error::deserialize(&mut deserializer)
            .map_err(json_deserialization_error)?;

        deserializer
            .end()
            .map_err(|error| bad_request(format!("invalid JSON syntax: {error}")))?;

        Ok(Self(value))
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Result<Response> {
        (
            [(CONTENT_TYPE, HeaderValue::from_static("application/json"))],
            serde_json::to_vec(&self.0)?,
        )
            .into_response()
    }
}

/// Returns whether `content_type` denotes a JSON payload: either
/// `application/json` or any `application/*+json` suffixed media type. Media
/// type parameters (such as `; charset=utf-8`) and case are ignored.
fn json_content_type(content_type: Option<&str>) -> bool {
    let Some(content_type) = content_type else {
        return false;
    };

    let content_type = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    content_type == "application/json"
        || content_type
            .strip_prefix("application/")
            .is_some_and(|subtype| subtype.ends_with("+json"))
}

#[allow(clippy::needless_pass_by_value)]
fn json_deserialization_error(error: serde_path_to_error::Error<serde_json::Error>) -> Error {
    let description = match error.inner().classify() {
        serde_json::error::Category::Data => {
            format!("invalid JSON value: {}", error.inner())
        }
        serde_json::error::Category::Syntax | serde_json::error::Category::Eof => {
            format!("invalid JSON syntax: {}", error.inner())
        }
        serde_json::error::Category::Io => {
            format!("failed to read JSON body: {}", error.inner())
        }
    };

    bad_request_at(error.path(), description).into()
}

#[cfg(test)]
mod tests {
    use http::{Request, header::CONTENT_TYPE};
    use serde_json::{Value, json};
    use topcoat_core::runtime::context::Cx;

    use super::*;
    use crate::runtime::{BadRequestError, Body, FromRequest, OptionalFromRequest, to_bytes};

    /// Builds a `Cx` carrying request `Parts` with the given `Content-Type`
    /// header, or no header at all when `content_type` is [`None`].
    fn cx_with_content_type(content_type: Option<&str>) -> Cx {
        let mut builder = Request::builder();
        if let Some(content_type) = content_type {
            builder = builder.header(CONTENT_TYPE, content_type);
        }

        let (parts, ()) = builder.body(()).expect("request should build").into_parts();

        let mut cx = Cx::empty();
        cx.insert(parts);
        cx
    }

    #[tokio::test]
    async fn from_request_deserializes_json_body() {
        let cx = cx_with_content_type(Some("application/json"));
        let Json(value) = <Json<Value> as FromRequest>::from_request(&cx, Body::from(r#"{"a":1}"#))
            .await
            .expect("a valid JSON body");

        assert_eq!(value, json!({ "a": 1 }));
    }

    #[tokio::test]
    async fn from_request_accepts_suffixed_json_content_type() {
        let cx = cx_with_content_type(Some("application/ld+json; charset=utf-8"));
        let result = <Json<Value> as FromRequest>::from_request(&cx, Body::from("true")).await;

        assert_eq!(result.expect("a valid JSON body").0, json!(true));
    }

    #[tokio::test]
    async fn from_request_without_json_content_type_is_bad_request() {
        for content_type in [None, Some("text/plain")] {
            let cx = cx_with_content_type(content_type);
            let error = <Json<Value> as FromRequest>::from_request(&cx, Body::from("{}"))
                .await
                .expect_err("a non-JSON content type is rejected");

            assert!(error.downcast_ref::<BadRequestError>().is_some());
        }
    }

    #[tokio::test]
    async fn from_request_rejects_trailing_data() {
        let cx = cx_with_content_type(Some("application/json"));
        let error = <Json<Value> as FromRequest>::from_request(&cx, Body::from(r#"{"a":1} oops"#))
            .await
            .expect_err("trailing data after the JSON value is rejected");

        assert!(error.downcast_ref::<BadRequestError>().is_some());
    }

    #[tokio::test]
    async fn optional_from_request_without_content_type_is_none() {
        let cx = cx_with_content_type(None);
        let json = <Json<Value> as OptionalFromRequest>::from_request(&cx, Body::empty())
            .await
            .expect("an absent content type is not an error");

        assert!(json.is_none());
    }

    #[tokio::test]
    async fn optional_from_request_with_content_type_is_some() {
        let cx = cx_with_content_type(Some("application/json"));
        let json = <Json<Value> as OptionalFromRequest>::from_request(&cx, Body::from("[1,2]"))
            .await
            .expect("a valid JSON body is not an error");

        assert_eq!(json.expect("a JSON payload is present").0, json!([1, 2]));
    }

    #[test]
    fn from_bytes_deserializes_into_target_type() {
        let Json(values) = Json::<Vec<i32>>::from_bytes(b"[1, 2, 3]").expect("valid JSON");
        assert_eq!(values, vec![1, 2, 3]);
    }

    #[test]
    fn from_bytes_reports_the_path_of_a_value_error() {
        let error = Json::<Vec<i32>>::from_bytes(br#"[1, "two", 3]"#)
            .expect_err("a type mismatch is rejected");

        let error = error
            .downcast_ref::<BadRequestError>()
            .expect("a bad-request error");
        assert_eq!(error.path(), Some("[1]"));
    }

    #[test]
    fn from_bytes_rejects_invalid_syntax() {
        let error = Json::<Value>::from_bytes(b"{").expect_err("malformed JSON is rejected");
        assert!(error.downcast_ref::<BadRequestError>().is_some());
    }

    #[tokio::test]
    async fn into_response_serializes_json_with_content_type() {
        let response = Json(json!({ "a": 1 }))
            .into_response()
            .expect("serialization succeeds");

        assert_eq!(
            response
                .headers()
                .get(CONTENT_TYPE)
                .map(http::HeaderValue::as_bytes),
            Some(b"application/json".as_slice())
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("reading the response body");
        assert_eq!(&body[..], br#"{"a":1}"#);
    }

    #[test]
    fn json_content_type_recognizes_json_media_types() {
        assert!(json_content_type(Some("application/json")));
        assert!(json_content_type(Some("application/json; charset=utf-8")));
        assert!(json_content_type(Some("APPLICATION/JSON")));
        assert!(json_content_type(Some("application/ld+json")));
        assert!(json_content_type(Some("application/vnd.api+json")));

        assert!(!json_content_type(None));
        assert!(!json_content_type(Some("text/plain")));
        assert!(!json_content_type(Some("application/xml")));
        assert!(!json_content_type(Some("text/json")));
    }
}
