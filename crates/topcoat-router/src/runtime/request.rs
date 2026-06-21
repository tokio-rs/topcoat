use topcoat_core::runtime::{context::Cx, error::Result};

use crate::runtime::{Body, error::bad_request, to_bytes};

pub use bytes::{Bytes, BytesMut};

/// An incoming HTTP request, carrying a [`Body`] by default.
pub type Request<T = Body> = http::Request<T>;

pub trait FromRequest: Sized {
    fn from_request(cx: &Cx, body: Body) -> impl Future<Output = Result<Self>> + Send;
}

impl FromRequest for Body {
    async fn from_request(_cx: &Cx, body: Body) -> Result<Self> {
        Ok(body)
    }
}

impl FromRequest for Bytes {
    async fn from_request(_cx: &Cx, body: Body) -> Result<Self> {
        to_bytes(body, usize::MAX)
            .await
            .map_err(|error| bad_request(format!("failed to read request body: {error}")).into())
    }
}

impl FromRequest for BytesMut {
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        let bytes = Bytes::from_request(cx, body).await?;
        Ok(Self::from(&bytes[..]))
    }
}

impl FromRequest for String {
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        let bytes = Bytes::from_request(cx, body).await?;
        Self::from_utf8(bytes.into()).map_err(|error| {
            bad_request(format!("request body is not valid UTF-8: {error}")).into()
        })
    }
}

/// Customizes the behavior of `Option<Self>` as a [`FromRequest`] extractor.
///
/// Implementing this trait lets `Option<Self>` be extracted from a request,
/// yielding `None` when the request carries no value for the extractor (for
/// example, a missing body) while still surfacing an error for values that are
/// present but malformed.
pub trait OptionalFromRequest: Sized {
    fn from_request(cx: &Cx, body: Body) -> impl Future<Output = Result<Option<Self>>> + Send;
}

impl<T> FromRequest for Option<T>
where
    T: OptionalFromRequest,
{
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        T::from_request(cx, body).await
    }
}
