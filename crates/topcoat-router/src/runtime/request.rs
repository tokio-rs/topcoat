use std::sync::Arc;

use axum::extract::{FromRequestParts, RawPathParams};
use topcoat_core::runtime::{
    context::{ContextMap, Cx},
    error::Result,
};

use crate::runtime::error::{BadRequestError, bad_request};

pub use axum::body::to_bytes;
pub use bytes::{Bytes, BytesMut};

pub type Body = axum::body::Body;

pub(crate) struct CxBody {
    pub(crate) cx: Cx,
    pub(crate) body: Body,
}

impl axum::extract::FromRequest<Arc<ContextMap>> for CxBody {
    type Rejection = BadRequestError;

    async fn from_request(
        req: axum::extract::Request,
        context: &Arc<ContextMap>,
    ) -> Result<Self, Self::Rejection> {
        let app_context = context.clone();
        let (mut parts, body) = req.into_parts();
        let body = Body::from(body);

        let mut request_context = ContextMap::new();
        request_context.register(
            RawPathParams::from_request_parts(&mut parts, context)
                .await
                .map_err(|error| bad_request(error.to_string()))?,
        );
        request_context.register(parts);

        let cx = Cx::new(app_context, request_context);
        Ok(Self { cx, body })
    }
}

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
