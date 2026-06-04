use std::ops::{Deref, DerefMut};

use ::serde::{Serialize, de::DeserializeOwned};
use axum::body::to_bytes;
use http::{
    HeaderMap, Method,
    header::{CONTENT_TYPE, HeaderValue},
};
use topcoat_core::{context::Cx, error::Result};

use crate::{Body, FromRequest, IntoResponse, bad_request, bad_request_at, headers, method, uri};

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
        if matches!(method(cx), &Method::GET | &Method::HEAD) {
            return Self::from_bytes(uri(cx).query().unwrap_or_default().as_bytes());
        }

        if !form_content_type(headers(cx)) {
            return Err(bad_request(
                "expected request with `Content-Type: application/x-www-form-urlencoded`",
            )
            .into());
        }

        let bytes = to_bytes(body, usize::MAX)
            .await
            .map_err(|error| bad_request(format!("failed to read request body: {error}")))?;

        Self::from_bytes(&bytes)
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
    fn into_response(self) -> Result<crate::Response> {
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

fn form_content_type(headers: &HeaderMap) -> bool {
    let Some(content_type) = headers.get(CONTENT_TYPE) else {
        return false;
    };

    let Ok(content_type) = content_type.to_str() else {
        return false;
    };

    content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .eq_ignore_ascii_case("application/x-www-form-urlencoded")
}
