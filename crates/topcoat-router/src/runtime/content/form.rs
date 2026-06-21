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
