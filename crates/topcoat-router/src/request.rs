//! Reading requests: the [`FromRequest`] extractor trait and accessors for
//! the parts of the current request, such as its [`uri`] and [`headers`].
//!
//! See the [`content`](crate::content) module for the typed request bodies,
//! such as JSON and forms.

use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

/// Byte buffers from the [`bytes`](https://docs.rs/bytes) crate. Both work as
/// request body extractors and as response bodies.
pub use bytes::{Bytes, BytesMut};
use http::request::Parts;
use topcoat_core::{
    context::{Cx, request_context, try_request_context},
    error::Result,
    identity::Identity,
};

use crate::{Body, RemoteAddr, body_limit, error::bad_request, proxy::ClientIp, to_bytes};

/// An incoming HTTP request. The body is a [`Body`] unless set otherwise.
pub type Request<T = Body> = http::Request<T>;

/// A type that can be built from an incoming request.
///
/// A page or route handler can take one `FromRequest` value as a parameter,
/// next to an optional `cx: &Cx`. The body is a stream that can only be read
/// once, so a handler can have at most one `FromRequest` parameter. The
/// built-in extractors include [`Json`](crate::content::Json),
/// [`Form`](crate::content::Form), [`Bytes`], [`String`], and [`Body`].
/// Implement the trait yourself for parsing the built-ins do not cover.
///
/// Wrap an extractor that implements [`OptionalFromRequest`] in [`Option`] to
/// make it optional.
///
/// An implementation that buffers the body should read it through [`Bytes`],
/// which enforces the request's [`body_limit`]. Reading the body by hand skips
/// that limit unless you pass it to [`to_bytes`].
///
/// # Examples
///
/// This extractor checks the JSON body against an `x-signature` header before
/// it deserializes it:
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
    /// # Errors
    ///
    /// Returns an error, usually a [`bad_request`], when the request cannot be
    /// parsed into `Self`. The router turns the error into the response sent
    /// to the client.
    fn from_request(cx: &Cx, body: Body) -> impl Future<Output = Result<Self>> + Send;
}

/// Returns the request body as is, so the handler can stream or forward it.
/// The body limit does not apply.
impl FromRequest for Body {
    fn from_request(_cx: &Cx, body: Body) -> impl Future<Output = Result<Self>> {
        core::future::ready(Ok(body))
    }
}

/// Reads the whole request body into memory. A body longer than the request's
/// [`body_limit`] is rejected with `413 Content Too Large`.
impl FromRequest for Bytes {
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        to_bytes(body, body_limit(cx)).await
    }
}

/// Reads the whole request body into a mutable buffer, like [`Bytes`] does.
impl FromRequest for BytesMut {
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        let bytes = Bytes::from_request(cx, body).await?;
        Ok(Self::from(&bytes[..]))
    }
}

/// Reads the whole request body as UTF-8 text, like [`Bytes`] does. A body that
/// is not valid UTF-8 is rejected with `400 Bad Request`.
impl FromRequest for String {
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        let bytes = Bytes::from_request(cx, body).await?;
        Self::from_utf8(bytes.into()).map_err(|error| {
            bad_request(format!("request body is not valid UTF-8: {error}")).into()
        })
    }
}

/// An extractor that can be made optional by wrapping it in [`Option`].
///
/// Implementing this trait makes `Option<Self>` a [`FromRequest`] extractor.
/// It yields `None` when the request has no value for the extractor, for
/// example when a body is missing. It still returns an error for a value that
/// is present but malformed.
pub trait OptionalFromRequest: Sized {
    /// Builds `Some(Self)` from the request, or returns `None` when the request
    /// has no value for this extractor.
    ///
    /// # Errors
    ///
    /// Returns an error when a value is present but malformed.
    fn from_request(cx: &Cx, body: Body) -> impl Future<Output = Result<Option<Self>>> + Send;
}

/// Makes any [`OptionalFromRequest`] extractor optional. See
/// [`OptionalFromRequest`] for when it yields `None`.
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
/// Use it to read several parts of the request at once. For a single part,
/// the accessors such as [`method`], [`uri`], and [`headers`] are shorter.
///
/// These are the parts of the request as the router currently handles it. A
/// [`rewrite`](crate::error::rewrite) or a layer can change them. Use
/// [`original_parts`] for the request as the client sent it.
///
/// # Panics
///
/// Panics if the context does not belong to a request, for example outside
/// the router.
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
/// # Panics
///
/// Panics if the context does not belong to a request, like [`parts`].
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
/// # Panics
///
/// Panics if the context does not belong to a request, like [`parts`].
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

/// Returns the HTTP [`Version`] of the current request.
///
/// [`Version`]: http::Version
///
/// # Panics
///
/// Panics if the context does not belong to a request, like [`parts`].
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
/// # Panics
///
/// Panics if the context does not belong to a request, like [`parts`].
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
/// # Panics
///
/// Panics if the context does not belong to a request, like [`parts`].
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
/// Extensions hold typed values attached to the request, usually by the server
/// or by middleware that runs before the handler.
///
/// [`Extensions`]: http::Extensions
///
/// # Panics
///
/// Panics if the context does not belong to a request, like [`parts`].
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

/// Returns the IP address and port of the direct connection for this request,
/// or `None` when they are unknown.
///
/// Behind a reverse proxy, this is the address of the proxy. Use [`client_ip`]
/// to read the address of the client instead.
///
/// Returns `None` if the request has no [`RemoteAddr`] in its extensions, as
/// is usual for Unix socket connections, or if the router is not handling the
/// current request.
///
/// # Examples
///
/// ```rust
/// use topcoat::{context::Cx, router::request::remote_addr};
///
/// fn peer_port(cx: &Cx) -> Option<u16> {
///     remote_addr(cx).map(|addr| addr.port())
/// }
/// ```
#[inline]
#[must_use]
pub fn remote_addr(cx: &Cx) -> Option<SocketAddr> {
    let parts = try_request_context::<Parts>(cx)?;
    parts.extensions.get::<RemoteAddr>().map(|remote| remote.0)
}

/// Returns the IP address of the client for this request, or `None` when it
/// cannot be determined.
///
/// By default, this is the IP address from [`remote_addr`]. Behind a reverse
/// proxy, that is the address of the proxy. Configure
/// [`TrustedProxies`](crate::TrustedProxies) on the router to read the
/// address of the client from a header that the proxy sets.
///
/// For a header that lists several addresses, Topcoat starts with the direct
/// connection and reads the list from right to left. It skips trusted proxies
/// and returns the first address it does not trust. If every address is
/// trusted, it returns the leftmost address. If the list is empty or missing,
/// it uses the address of the direct connection.
///
/// Returns `None` if the address of the direct connection is unknown and not
/// trusted through [`TrustedProxies::nearest`](crate::TrustedProxies::nearest),
/// or if an address needed from the header cannot be parsed. Headers that hold
/// a single address follow the rules in
/// [`ForwardedHeader::Single`](crate::ForwardedHeader::Single). IPv4-mapped
/// IPv6 addresses are returned as IPv4.
///
/// The router determines the address before it runs any layers, so later
/// changes to the request headers do not change the result. Returns `None` if
/// the router is not handling the current request.
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{error::forbidden, request::client_ip},
/// };
///
/// # fn is_banned(_ip: std::net::IpAddr) -> bool { false }
/// fn reject_banned(cx: &Cx) -> Result<()> {
///     match client_ip(cx) {
///         Some(ip) if is_banned(ip) => Err(forbidden().into()),
///         _ => Ok(()),
///     }
/// }
/// ```
#[must_use]
#[track_caller]
pub fn client_ip(cx: &Cx) -> Option<IpAddr> {
    try_request_context::<ClientIp>(cx)?.0
}

/// The parts the request arrived with, shared by every dispatch.
#[derive(Debug, Clone)]
pub(crate) struct OriginalParts(pub(crate) Arc<Parts>);

/// Returns the [`Parts`] of the request as the client sent it, before any
/// rewrite or change made by a layer.
///
/// A handler reached through a [`rewrite`](crate::error::rewrite) sees the
/// rewritten request in [`parts`], which can have a different URI and method.
/// This function returns the parts the request arrived with. Layers can also
/// change the current parts without a rewrite. For example,
/// [`StripPrefixLayer`](crate::StripPrefixLayer) changes the current URI but
/// not the original one.
///
/// Outside the router, this is the same as [`parts`].
///
/// # Examples
///
/// ```rust
/// use topcoat::{context::Cx, router::request::original_parts};
///
/// async fn arrived_with_header(cx: &Cx, name: &str) -> bool {
///     original_parts(cx).headers.contains_key(name)
/// }
/// ```
#[inline]
#[must_use]
#[track_caller]
pub fn original_parts(cx: &Cx) -> &Parts {
    match try_request_context::<OriginalParts>(cx) {
        Some(original) => &original.0,
        None => parts(cx),
    }
}

/// Returns the HTTP [`Method`] of the request as the client sent it, before
/// any rewrite.
///
/// A rewrite can change the method of the request. This function returns the
/// method the request arrived with. For a request that was never rewritten,
/// it is the same as [`method`].
///
/// [`Method`]: http::Method
///
/// # Examples
///
/// ```rust
/// use topcoat::{context::Cx, router::request::original_method};
///
/// async fn arrived_as_post(cx: &Cx) -> bool {
///     original_method(cx) == http::Method::POST
/// }
/// ```
#[inline]
#[must_use]
#[track_caller]
pub fn original_method(cx: &Cx) -> &http::Method {
    &original_parts(cx).method
}

/// Returns the [`Uri`] of the request as the client sent it, before any
/// rewrite or change made by a layer.
///
/// A handler reached through a [`rewrite`](crate::error::rewrite) sees the
/// rewritten URI in [`uri`]. This function returns the URI the request
/// arrived with, which is the URL the browser shows. Use it, for example, to
/// render a form that posts back to that URL. Layers such as
/// [`StripPrefixLayer`](crate::StripPrefixLayer) can also change the current
/// URI without changing the original one.
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
    &original_parts(cx).uri
}

/// Returns the HTTP [`Version`] of the request as the client sent it.
///
/// See [`original_parts`] for how the current request can differ from the
/// one that arrived.
///
/// [`Version`]: http::Version
#[inline]
#[must_use]
#[track_caller]
pub fn original_version(cx: &Cx) -> &http::Version {
    &original_parts(cx).version
}

/// Returns the [`HeaderMap`] of the request as the client sent it.
///
/// See [`original_parts`] for how the current request can differ from the
/// one that arrived.
///
/// [`HeaderMap`]: http::HeaderMap
#[inline]
#[must_use]
#[track_caller]
pub fn original_headers(cx: &Cx) -> &http::HeaderMap {
    &original_parts(cx).headers
}

/// Returns the `Content-Type` header of the request as the client sent it, or
/// [`None`] when it is absent or not valid UTF-8.
///
/// See [`original_parts`] for how the current request can differ from the
/// one that arrived.
#[inline]
#[must_use]
#[track_caller]
pub fn original_content_type(cx: &Cx) -> Option<&str> {
    original_headers(cx)
        .get(http::header::CONTENT_TYPE)?
        .to_str()
        .ok()
}

/// Returns the [`Extensions`] of the request as the client sent it.
///
/// See [`original_parts`] for how the current request can differ from the
/// one that arrived.
///
/// [`Extensions`]: http::Extensions
#[inline]
#[must_use]
#[track_caller]
pub fn original_extensions(cx: &Cx) -> &http::Extensions {
    &original_parts(cx).extensions
}

/// The name of the request header that holds the starting identity of a
/// request. See [`initial_identity`].
pub const IDENTITY_HEADER: &str = "x-topcoat-identity";

/// Returns the identity that rendering starts at for the current request.
///
/// When a client renders part of a page again, it sends the identity of that
/// part in the [`IDENTITY_HEADER`]. The server then derives the same
/// identities inside that part as in the page the client already has. A
/// request without the header starts at [`Identity::ROOT`], like a page
/// request.
///
/// # Errors
///
/// Returns a `400 Bad Request` error if the header is present but does not
/// hold a valid identity.
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result, context::Cx, core::identity::Identity, router::request::initial_identity,
/// };
///
/// async fn is_page_request(cx: &Cx) -> Result<bool> {
///     Ok(initial_identity(cx)? == Identity::ROOT)
/// }
/// ```
#[track_caller]
pub fn initial_identity(cx: &Cx) -> Result<Identity> {
    let Some(header) = headers(cx).get(IDENTITY_HEADER) else {
        return Ok(Identity::ROOT);
    };
    header
        .to_str()
        .ok()
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| {
            bad_request(format!("expected `{IDENTITY_HEADER}` to be an identity")).into()
        })
}
