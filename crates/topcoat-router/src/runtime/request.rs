use topcoat_core::runtime::{context::Cx, error::Result};

use crate::runtime::{Body, error::bad_request, to_bytes};

/// Byte-buffer types re-exported for use as request body extractors and as
/// response bodies.
pub use bytes::{Bytes, BytesMut};

/// An incoming HTTP request, carrying a [`Body`] by default.
pub type Request<T = Body> = http::Request<T>;

/// A type that can be built from an incoming request.
///
/// A page or route handler may take a single `FromRequest` value as its request
/// body parameter, optionally alongside `cx: &Cx`. The built-in extractors
/// ([`Json`](crate::runtime::Json), [`Form`](crate::runtime::Form), [`Bytes`],
/// [`String`], [`Body`], and more) all implement this trait; implement it
/// yourself for request-specific parsing the built-ins don't cover.
///
/// Because the body is a stream that can only be read once, a handler may have
/// at most one `FromRequest` parameter. This is the request-side counterpart of
/// [`IntoResponse`](crate::runtime::IntoResponse).
///
/// # Examples
///
/// Implement it to parse a request in a way the built-ins don't cover. Here,
/// JSON whose body is verified against an `x-signature` header before it is
/// deserialized:
///
/// ```rust
/// # #[derive(serde::Deserialize)]
/// # struct CreateUser { name: String }
/// # fn verify_signature(_signature: &str, _bytes: &[u8]) -> topcoat::Result<()> { Ok(()) }
/// use serde::de::DeserializeOwned;
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{Body, FromRequest, bad_request, headers, route, to_bytes},
/// };
///
/// struct SignedJson<T>(T);
///
/// impl<T> FromRequest for SignedJson<T>
/// where
///     T: DeserializeOwned,
/// {
///     async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
///         let signature = headers(cx)
///             .get("x-signature")
///             .and_then(|value| value.to_str().ok())
///             .ok_or_else(|| bad_request("missing x-signature header"))?;
///
///         let bytes = to_bytes(body, usize::MAX)
///             .await
///             .map_err(|error| bad_request(format!("failed to read body: {error}")))?;
///
///         verify_signature(signature, &bytes)?;
///
///         Ok(Self(serde_json::from_slice(&bytes)?))
///     }
/// }
///
/// // Once implemented, use it like the built-in extractors:
/// #[route(POST "/api/signed")]
/// async fn signed(SignedJson(input): SignedJson<CreateUser>) -> Result<&'static str> {
///     let _ = input;
///     Ok("ok")
/// }
/// ```
pub trait FromRequest: Sized {
    /// Builds `Self` from the request context and body.
    ///
    /// Returns an error (typically [`bad_request`]) when the request cannot be
    /// parsed into `Self`; the error is converted into the response sent to the
    /// client.
    fn from_request(cx: &Cx, body: Body) -> impl Future<Output = Result<Self>> + Send;
}

/// Yields the request body unchanged, leaving it unbuffered for the handler to
/// read or forward itself.
impl FromRequest for Body {
    async fn from_request(_cx: &Cx, body: Body) -> Result<Self> {
        Ok(body)
    }
}

/// Buffers the entire request body into memory.
impl FromRequest for Bytes {
    async fn from_request(_cx: &Cx, body: Body) -> Result<Self> {
        to_bytes(body, usize::MAX)
            .await
            .map_err(|error| bad_request(format!("failed to read request body: {error}")).into())
    }
}

/// Buffers the entire request body into a mutable buffer.
impl FromRequest for BytesMut {
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        let bytes = Bytes::from_request(cx, body).await?;
        Ok(Self::from(&bytes[..]))
    }
}

/// Buffers the request body and decodes it as UTF-8, rejecting a non-UTF-8 body
/// with `400 Bad Request`.
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
    /// Builds `Some(Self)` from the request, or `None` when the request carries
    /// no value for this extractor.
    ///
    /// Returns an error only when a value is present but malformed.
    fn from_request(cx: &Cx, body: Body) -> impl Future<Output = Result<Option<Self>>> + Send;
}

/// Makes any [`OptionalFromRequest`] extractor optional, yielding `None` when
/// the request carries no value of that kind while still surfacing an error for
/// a value that is present but malformed.
impl<T> FromRequest for Option<T>
where
    T: OptionalFromRequest,
{
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        T::from_request(cx, body).await
    }
}
