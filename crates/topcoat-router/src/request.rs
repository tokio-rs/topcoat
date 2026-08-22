/// Byte-buffer types re-exported for use as request body extractors and as
/// response bodies.
pub use bytes::{Bytes, BytesMut};
use http::request::Parts;
use topcoat_core::{
    context::{Cx, request_context, try_request_context},
    error::Result,
};

use crate::{Body, body_limit, error::bad_request, to_bytes};

/// An incoming HTTP request, carrying a [`Body`] by default.
pub type Request<T = Body> = http::Request<T>;

/// A type that can be built from an incoming request.
///
/// A page or route handler may take a single `FromRequest` value as its request
/// body parameter, optionally alongside `cx: &Cx`. The built-in extractors
/// ([`Json`](crate::content::Json), [`Form`](crate::content::Form), [`Bytes`],
/// [`String`], [`Body`], and more) all implement this trait; implement it
/// yourself for request-specific parsing the built-ins don't cover.
///
/// Because the body is a stream that can only be read once, a handler may have
/// at most one `FromRequest` parameter. This is the request-side counterpart of
/// [`IntoResponse`](crate::response::IntoResponse).
///
/// An implementation that buffers the body should delegate the buffering to
/// [`Bytes`], which enforces the request's
/// [`body_limit`]; reading the body by hand bypasses that
/// limit.
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
///     router::{
///         Body,
///         error::bad_request,
///         request::{Bytes, FromRequest, headers},
///         route,
///     },
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
///         let bytes = Bytes::from_request(cx, body).await?;
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
    /// Returns an error (typically [`bad_request`])
    /// when the request cannot be parsed into `Self`; the error is converted
    /// into the response sent to the client.
    fn from_request(cx: &Cx, body: Body) -> impl Future<Output = Result<Self>> + Send;
}

/// Yields the request body unchanged, leaving it unbuffered for the handler to
/// read or forward itself.
impl FromRequest for Body {
    fn from_request(_cx: &Cx, body: Body) -> impl Future<Output = Result<Self>> {
        core::future::ready(Ok(body))
    }
}

/// Buffers the entire request body into memory, rejecting a body larger than
/// the request's [`body_limit`] with `413 Content Too Large`.
impl FromRequest for Bytes {
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        to_bytes(body, body_limit(cx)).await
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

/// Returns the [`Parts`] of the current request.
///
/// Use this when you need access to multiple components of the request at
/// once. For individual fields, prefer the dedicated accessors
/// ([`method`], [`uri`], [`version`], [`headers`], [`extensions`]).
///
/// # Examples
///
/// ```rust
/// use topcoat::{context::Cx, router::request::parts};
///
/// async fn log_request(cx: &Cx) {
///     let parts = parts(cx);
///     println!("{} {}", parts.method, parts.uri);
/// }
/// ```
#[inline]
#[must_use]
#[track_caller]
pub fn parts(cx: &Cx) -> &Parts {
    request_context(cx)
}

/// Returns the HTTP [`Method`] of the current request.
///
/// [`Method`]: http::Method
///
/// # Examples
///
/// ```rust
/// use topcoat::{context::Cx, router::request::method};
///
/// async fn is_post(cx: &Cx) -> bool {
///     method(cx) == http::Method::POST
/// }
/// ```
#[inline]
#[must_use]
#[track_caller]
pub fn method(cx: &Cx) -> &http::Method {
    &parts(cx).method
}

/// Returns the [`Uri`] of the current request.
///
/// [`Uri`]: http::Uri
///
/// # Examples
///
/// ```rust
/// use topcoat::{context::Cx, router::request::uri};
///
/// async fn current_path(cx: &Cx) -> &str {
///     uri(cx).path()
/// }
/// ```
#[inline]
#[must_use]
#[track_caller]
pub fn uri(cx: &Cx) -> &http::Uri {
    &parts(cx).uri
}

/// The URI a rewritten request originally arrived with, stored on the request
/// context of every dispatch reached through a rewrite.
#[derive(Debug, Clone)]
pub(crate) struct OriginalUri(pub(crate) http::Uri);

/// Returns the [`Uri`] the client actually requested, before any rewrite.
///
/// A handler reached through a [`rewrite`](crate::error::rewrite) sees the
/// rewritten URI in [`uri`]; this accessor returns the URI the request
/// arrived with, for example to render a form that posts back to the visible
/// URL. For a request that was never rewritten the two are the same.
///
/// [`Uri`]: http::Uri
///
/// # Examples
///
/// ```rust
/// use topcoat::{context::Cx, router::request::original_uri};
///
/// async fn form_action(cx: &Cx) -> String {
///     original_uri(cx).path().to_owned()
/// }
/// ```
#[inline]
#[must_use]
#[track_caller]
pub fn original_uri(cx: &Cx) -> &http::Uri {
    match try_request_context::<OriginalUri>(cx) {
        Some(original) => &original.0,
        None => uri(cx),
    }
}

/// Returns the HTTP [`Version`] of the current request.
///
/// [`Version`]: http::Version
///
/// # Examples
///
/// ```rust
/// use topcoat::{context::Cx, router::request::version};
///
/// async fn is_http2(cx: &Cx) -> bool {
///     *version(cx) == http::Version::HTTP_2
/// }
/// ```
#[inline]
#[must_use]
#[track_caller]
pub fn version(cx: &Cx) -> &http::Version {
    &parts(cx).version
}

/// Returns the [`HeaderMap`] of the current request.
///
/// [`HeaderMap`]: http::HeaderMap
///
/// # Examples
///
/// ```rust
/// use topcoat::{context::Cx, router::request::headers};
///
/// async fn user_agent(cx: &Cx) -> Option<&str> {
///     headers(cx).get("user-agent")?.to_str().ok()
/// }
/// ```
#[inline]
#[must_use]
#[track_caller]
pub fn headers(cx: &Cx) -> &http::HeaderMap {
    &parts(cx).headers
}

/// Returns the `Content-Type` header of the current request as a string slice,
/// or [`None`] when it is absent or not valid UTF-8.
///
/// # Examples
///
/// ```rust
/// use topcoat::{context::Cx, router::request::content_type};
///
/// async fn is_json(cx: &Cx) -> bool {
///     content_type(cx).is_some_and(|value| value.starts_with("application/json"))
/// }
/// ```
#[inline]
#[must_use]
#[track_caller]
pub fn content_type(cx: &Cx) -> Option<&str> {
    headers(cx).get(http::header::CONTENT_TYPE)?.to_str().ok()
}

/// Returns the [`Extensions`] of the current request.
///
/// Extensions carry typed values attached to the request, typically by
/// middleware running before the handler.
///
/// [`Extensions`]: http::Extensions
///
/// # Examples
///
/// ```rust
/// use topcoat::{context::Cx, router::request::extensions};
///
/// struct RequestId(String);
///
/// async fn request_id(cx: &Cx) -> Option<&str> {
///     extensions(cx).get::<RequestId>().map(|id| id.0.as_str())
/// }
/// ```
#[inline]
#[must_use]
#[track_caller]
pub fn extensions(cx: &Cx) -> &http::Extensions {
    &parts(cx).extensions
}
