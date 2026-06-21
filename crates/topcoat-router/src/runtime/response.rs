use std::borrow::Cow;

use bytes::Bytes;
use http::header::{CONTENT_TYPE, HeaderName, HeaderValue};
use http::StatusCode;
use topcoat_core::runtime::context::Cx;
use topcoat_core::runtime::error::Result;

use crate::runtime::Body;

pub type Response<T = Body> = http::Response<T>;

const TEXT_PLAIN: HeaderValue = HeaderValue::from_static("text/plain; charset=utf-8");
const APPLICATION_OCTET_STREAM: HeaderValue = HeaderValue::from_static("application/octet-stream");

/// Applies request-scoped side effects to a finished response before it is sent.
///
/// Currently this writes any pending cookie changes (with the `cookie` feature)
/// onto the response as `Set-Cookie` headers. It is a no-op otherwise.
#[inline]
pub(crate) fn finalize(cx: &Cx, response: Response) -> Response {
    #[cfg(feature = "cookie")]
    {
        let mut response = response;
        topcoat_cookie::write_cookies(cx, response.headers_mut());
        response
    }
    #[cfg(not(feature = "cookie"))]
    {
        let _ = cx;
        response
    }
}

/// Converts a value into an HTTP [`Response`].
///
/// Handlers return any type that implements this trait, including the common
/// in-memory types (`&str`, `String`, `Vec<u8>`, [`Bytes`]), a bare
/// [`StatusCode`], a [`Response`] itself, and the framework's own wrappers
/// ([`Html`], [`Json`](crate::runtime::Json), [`Form`](crate::runtime::Form)).
/// Status codes and headers can be attached by wrapping a value in a tuple:
/// `(StatusCode, T)` or `([(HeaderName, HeaderValue); N], T)`.
pub trait IntoResponse {
    fn into_response(self) -> Result<Response>;
}

/// Builds a response carrying `body` with the given `Content-Type`.
pub(crate) fn content_response(content_type: HeaderValue, body: Body) -> Response {
    let mut response = Response::new(body);
    response.headers_mut().insert(CONTENT_TYPE, content_type);
    response
}

impl IntoResponse for Response {
    fn into_response(self) -> Result<Response> {
        Ok(self)
    }
}

impl IntoResponse for Body {
    fn into_response(self) -> Result<Response> {
        Ok(Response::new(self))
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Result<Response> {
        Ok(Response::new(Body::empty()))
    }
}

impl IntoResponse for StatusCode {
    fn into_response(self) -> Result<Response> {
        let mut response = Response::new(Body::empty());
        *response.status_mut() = self;
        Ok(response)
    }
}

impl IntoResponse for &'static str {
    fn into_response(self) -> Result<Response> {
        Ok(content_response(TEXT_PLAIN, Body::from(self)))
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Result<Response> {
        Ok(content_response(TEXT_PLAIN, Body::from(self)))
    }
}

impl IntoResponse for Cow<'static, str> {
    fn into_response(self) -> Result<Response> {
        match self {
            Cow::Borrowed(value) => value.into_response(),
            Cow::Owned(value) => value.into_response(),
        }
    }
}

impl IntoResponse for &'static [u8] {
    fn into_response(self) -> Result<Response> {
        Ok(content_response(APPLICATION_OCTET_STREAM, Body::from(self)))
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Result<Response> {
        Ok(content_response(APPLICATION_OCTET_STREAM, Body::from(self)))
    }
}

impl IntoResponse for Bytes {
    fn into_response(self) -> Result<Response> {
        Ok(content_response(APPLICATION_OCTET_STREAM, Body::from(self)))
    }
}

/// Sets the status code of the response produced by the inner value.
impl<T> IntoResponse for (StatusCode, T)
where
    T: IntoResponse,
{
    fn into_response(self) -> Result<Response> {
        let (status, inner) = self;
        let mut response = inner.into_response()?;
        *response.status_mut() = status;
        Ok(response)
    }
}

/// Inserts the given headers onto the response produced by the inner value.
impl<T, const N: usize> IntoResponse for ([(HeaderName, HeaderValue); N], T)
where
    T: IntoResponse,
{
    fn into_response(self) -> Result<Response> {
        let (headers, inner) = self;
        let mut response = inner.into_response()?;
        for (name, value) in headers {
            response.headers_mut().insert(name, value);
        }
        Ok(response)
    }
}
