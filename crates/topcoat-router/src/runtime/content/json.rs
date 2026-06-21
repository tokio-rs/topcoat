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
