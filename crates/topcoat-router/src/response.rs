use std::{borrow::Cow, convert::Infallible};

use bytes::{Bytes, BytesMut};
use http::{
    Extensions, HeaderMap, StatusCode,
    header::{CONTENT_TYPE, HeaderName, HeaderValue},
    response::Parts,
};
use topcoat_core::{
    context::Cx,
    error::{Error, Result},
};

use crate::{Body, BoxError};

pub type Response<T = Body> = http::Response<T>;

const TEXT_PLAIN: HeaderValue = HeaderValue::from_static("text/plain; charset=utf-8");
const APPLICATION_OCTET_STREAM: HeaderValue = HeaderValue::from_static("application/octet-stream");

/// Converts a value into an HTTP [`Response`].
///
/// Route handlers return any type that implements this trait.
/// The conversion receives the request [`Cx`], so a response can depend on
/// request-scoped state.
///
/// A response can also be assembled from a tuple. The last element is converted
/// with `IntoResponse` and becomes the body, while the earlier elements modify
/// the response: a leading [`StatusCode`], [`Parts`], or [`Response<()>`] sets
/// the status line, and every other element is applied with
/// [`IntoResponseParts`]. For example `(StatusCode::CREATED, headers, body)`
/// builds a `201` response carrying `headers` and `body`.
///
/// # Examples
///
/// Implement it for a domain type that should control its own status, headers,
/// or body:
///
/// ```rust
/// use topcoat::{
///     Result,
///     context::Cx,
///     router::{
///         Body,
///         response::{IntoResponse, Response},
///         route,
///     },
/// };
///
/// struct Csv(String);
///
/// impl IntoResponse for Csv {
///     async fn into_response(self, _cx: &Cx) -> Result<Response> {
///         Ok(Response::builder()
///             .header("Content-Type", "text/csv; charset=utf-8")
///             .body(Body::from(self.0))?)
///     }
/// }
///
/// #[route(GET "/api/report.csv")]
/// async fn report() -> Result<Csv> {
///     Ok(Csv("name,total\nAda,42\n".to_string()))
/// }
/// ```
pub trait IntoResponse {
    /// Converts `self` into an HTTP [`Response`], using the request [`Cx`] for any
    /// request-scoped data.
    ///
    /// # Errors
    ///
    /// Returns an error if the response cannot be assembled (for example, a
    /// header value is invalid).
    fn into_response(self, cx: &Cx) -> impl Future<Output = Result<Response>> + Send;
}

/// Modifies a [`Response`]'s [`Parts`] without supplying a body.
///
/// Types that implement this trait (header arrays, [`HeaderMap`],
/// [`Extensions`], and their [`Option`] wrappers) can appear before the final
/// body element of an [`IntoResponse`] tuple to attach headers or extensions to
/// the response. The conversion receives the request [`Cx`] so a part can
/// depend on request-scoped state.
pub trait IntoResponseParts {
    /// Applies `self` to the response `parts`, using the request [`Cx`] for any
    /// request-scoped data.
    ///
    /// # Errors
    ///
    /// Returns an error if a part cannot be applied (for example, a header
    /// value is invalid).
    fn into_response_parts(
        self,
        cx: &Cx,
        parts: &mut Parts,
    ) -> impl Future<Output = Result<()>> + Send;
}

/// Builds a response carrying `body` with the given `Content-Type`.
pub(crate) fn content_response(content_type: HeaderValue, body: Body) -> Response {
    let mut response = Response::new(body);
    response.headers_mut().insert(CONTENT_TYPE, content_type);
    response
}

/// Copies the status, version, headers, and extensions of `from` onto `parts`,
/// used when a tuple leads with a [`Parts`] or [`Response<()>`] template.
fn merge_parts(parts: &mut Parts, from: Parts) {
    parts.status = from.status;
    parts.version = from.version;
    parts.headers.extend(from.headers);
    parts.extensions.extend(from.extensions);
}

// -- Leaf IntoResponse impls --

impl IntoResponse for Infallible {
    async fn into_response(self, _cx: &Cx) -> Result<Response> {
        match self {}
    }
}

impl IntoResponse for () {
    async fn into_response(self, _cx: &Cx) -> Result<Response> {
        Ok(Response::new(Body::empty()))
    }
}

impl IntoResponse for StatusCode {
    async fn into_response(self, cx: &Cx) -> Result<Response> {
        (self, ()).into_response(cx).await
    }
}

impl IntoResponse for &'static str {
    async fn into_response(self, _cx: &Cx) -> Result<Response> {
        Ok(content_response(TEXT_PLAIN, Body::from(self)))
    }
}

impl IntoResponse for String {
    async fn into_response(self, _cx: &Cx) -> Result<Response> {
        Ok(content_response(TEXT_PLAIN, Body::from(self)))
    }
}

impl IntoResponse for Box<str> {
    async fn into_response(self, cx: &Cx) -> Result<Response> {
        String::from(self).into_response(cx).await
    }
}

impl IntoResponse for Cow<'static, str> {
    async fn into_response(self, cx: &Cx) -> Result<Response> {
        match self {
            Cow::Borrowed(value) => value.into_response(cx).await,
            Cow::Owned(value) => value.into_response(cx).await,
        }
    }
}

impl IntoResponse for &'static [u8] {
    async fn into_response(self, _cx: &Cx) -> Result<Response> {
        Ok(content_response(APPLICATION_OCTET_STREAM, Body::from(self)))
    }
}

impl<const N: usize> IntoResponse for &'static [u8; N] {
    async fn into_response(self, cx: &Cx) -> Result<Response> {
        let bytes: &'static [u8] = self;
        bytes.into_response(cx).await
    }
}

impl<const N: usize> IntoResponse for [u8; N] {
    async fn into_response(self, cx: &Cx) -> Result<Response> {
        self.to_vec().into_response(cx).await
    }
}

impl IntoResponse for Vec<u8> {
    async fn into_response(self, _cx: &Cx) -> Result<Response> {
        Ok(content_response(APPLICATION_OCTET_STREAM, Body::from(self)))
    }
}

impl IntoResponse for Box<[u8]> {
    async fn into_response(self, cx: &Cx) -> Result<Response> {
        Vec::from(self).into_response(cx).await
    }
}

impl IntoResponse for Cow<'static, [u8]> {
    async fn into_response(self, cx: &Cx) -> Result<Response> {
        match self {
            Cow::Borrowed(value) => value.into_response(cx).await,
            Cow::Owned(value) => value.into_response(cx).await,
        }
    }
}

impl IntoResponse for Bytes {
    async fn into_response(self, _cx: &Cx) -> Result<Response> {
        Ok(content_response(APPLICATION_OCTET_STREAM, Body::from(self)))
    }
}

impl IntoResponse for BytesMut {
    async fn into_response(self, cx: &Cx) -> Result<Response> {
        self.freeze().into_response(cx).await
    }
}

impl IntoResponse for Body {
    async fn into_response(self, _cx: &Cx) -> Result<Response> {
        Ok(Response::new(self))
    }
}

impl IntoResponse for HeaderMap {
    async fn into_response(self, cx: &Cx) -> Result<Response> {
        (self, ()).into_response(cx).await
    }
}

impl IntoResponse for Extensions {
    async fn into_response(self, cx: &Cx) -> Result<Response> {
        (self, ()).into_response(cx).await
    }
}

impl IntoResponse for Parts {
    async fn into_response(self, cx: &Cx) -> Result<Response> {
        (self, ()).into_response(cx).await
    }
}

/// Replies with an empty body carrying each `(name, value)` pair as a header.
impl<K, V, const N: usize> IntoResponse for [(K, V); N]
where
    K: TryInto<HeaderName> + Send,
    K::Error: std::error::Error + Send + Sync + 'static,
    V: TryInto<HeaderValue> + Send,
    V::Error: std::error::Error + Send + Sync + 'static,
{
    async fn into_response(self, cx: &Cx) -> Result<Response> {
        (self, ()).into_response(cx).await
    }
}

/// Re-bodies any [`http::Response`] whose body is a [`Bytes`] stream into the
/// framework's [`Body`], leaving the parts untouched.
impl<B> IntoResponse for http::Response<B>
where
    B: http_body::Body<Data = Bytes> + Send + 'static,
    B::Error: Into<BoxError>,
{
    async fn into_response(self, _cx: &Cx) -> Result<Response> {
        let (parts, body) = self.into_parts();
        Ok(Response::from_parts(parts, Body::new(body)))
    }
}

// -- IntoResponseParts impls --

impl IntoResponseParts for () {
    async fn into_response_parts(self, _cx: &Cx, _parts: &mut Parts) -> Result<()> {
        Ok(())
    }
}

impl<T> IntoResponseParts for Option<T>
where
    T: IntoResponseParts + Send,
{
    async fn into_response_parts(self, cx: &Cx, parts: &mut Parts) -> Result<()> {
        if let Some(value) = self {
            value.into_response_parts(cx, parts).await?;
        }
        Ok(())
    }
}

impl IntoResponseParts for HeaderMap {
    async fn into_response_parts(self, _cx: &Cx, parts: &mut Parts) -> Result<()> {
        parts.headers.extend(self);
        Ok(())
    }
}

impl IntoResponseParts for Extensions {
    async fn into_response_parts(self, _cx: &Cx, parts: &mut Parts) -> Result<()> {
        parts.extensions.extend(self);
        Ok(())
    }
}

/// Inserts each `(name, value)` pair as a response header, failing if a name or
/// value is not a valid header.
impl<K, V, const N: usize> IntoResponseParts for [(K, V); N]
where
    K: TryInto<HeaderName> + Send,
    K::Error: std::error::Error + Send + Sync + 'static,
    V: TryInto<HeaderValue> + Send,
    V::Error: std::error::Error + Send + Sync + 'static,
{
    async fn into_response_parts(self, _cx: &Cx, parts: &mut Parts) -> Result<()> {
        for (name, value) in self {
            let name = name.try_into().map_err(Error::from)?;
            let value = value.try_into().map_err(Error::from)?;
            parts.headers.insert(name, value);
        }
        Ok(())
    }
}

// -- Tuple IntoResponse impls --

/// Generates the four `IntoResponse` tuple families for a fixed number of
/// [`IntoResponseParts`] members `T1..Tn`: a bare tuple, and tuples led by a
/// [`StatusCode`], [`Parts`], or [`Response<()>`] that seeds the response.
///
/// In every family the final element is the body (any [`IntoResponse`]) and the
/// `Ti` are applied to the resulting [`Parts`] in order.
macro_rules! impl_into_response_tuples {
    ( $($ty:ident),* ) => {
        #[allow(non_snake_case, unused_mut, unused_variables)]
        impl<R, $($ty,)*> IntoResponse for ($($ty,)* R,)
        where
            R: IntoResponse + Send,
            $($ty: IntoResponseParts + Send,)*
        {
            async fn into_response(self, cx: &Cx) -> Result<Response> {
                let ($($ty,)* r,) = self;
                let (mut parts, body) = r.into_response(cx).await?.into_parts();
                $( $ty.into_response_parts(cx, &mut parts).await?; )*
                Ok(Response::from_parts(parts, body))
            }
        }

        #[allow(non_snake_case, unused_mut, unused_variables)]
        impl<R, $($ty,)*> IntoResponse for (StatusCode, $($ty,)* R,)
        where
            R: IntoResponse + Send,
            $($ty: IntoResponseParts + Send,)*
        {
            async fn into_response(self, cx: &Cx) -> Result<Response> {
                let (status, $($ty,)* r,) = self;
                let (mut parts, body) = r.into_response(cx).await?.into_parts();
                parts.status = status;
                $( $ty.into_response_parts(cx, &mut parts).await?; )*
                Ok(Response::from_parts(parts, body))
            }
        }

        #[allow(non_snake_case, unused_mut, unused_variables)]
        impl<R, $($ty,)*> IntoResponse for (Parts, $($ty,)* R,)
        where
            R: IntoResponse + Send,
            $($ty: IntoResponseParts + Send,)*
        {
            async fn into_response(self, cx: &Cx) -> Result<Response> {
                let (template, $($ty,)* r,) = self;
                let (mut parts, body) = r.into_response(cx).await?.into_parts();
                merge_parts(&mut parts, template);
                $( $ty.into_response_parts(cx, &mut parts).await?; )*
                Ok(Response::from_parts(parts, body))
            }
        }

        #[allow(non_snake_case, unused_mut, unused_variables)]
        impl<R, $($ty,)*> IntoResponse for (Response<()>, $($ty,)* R,)
        where
            R: IntoResponse + Send,
            $($ty: IntoResponseParts + Send,)*
        {
            async fn into_response(self, cx: &Cx) -> Result<Response> {
                let (template, $($ty,)* r,) = self;
                let (template, ()) = template.into_parts();
                let (mut parts, body) = r.into_response(cx).await?.into_parts();
                merge_parts(&mut parts, template);
                $( $ty.into_response_parts(cx, &mut parts).await?; )*
                Ok(Response::from_parts(parts, body))
            }
        }
    };
}

impl_into_response_tuples!();
impl_into_response_tuples!(T1);
impl_into_response_tuples!(T1, T2);
impl_into_response_tuples!(T1, T2, T3);
impl_into_response_tuples!(T1, T2, T3, T4);
impl_into_response_tuples!(T1, T2, T3, T4, T5);
impl_into_response_tuples!(T1, T2, T3, T4, T5, T6);
impl_into_response_tuples!(T1, T2, T3, T4, T5, T6, T7);
impl_into_response_tuples!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_into_response_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_into_response_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_into_response_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_into_response_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_into_response_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_into_response_tuples!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_into_response_tuples!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15
);
impl_into_response_tuples!(
    T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15, T16
);

#[cfg(test)]
mod tests {
    use http_body_util::Full;

    use super::*;
    use crate::to_bytes;

    /// Renders a value into a response and reads the body fully into memory.
    async fn run(value: impl IntoResponse) -> (Parts, Bytes) {
        let cx = Cx::default().await;
        let (parts, body) = value.into_response(&cx).await.unwrap().into_parts();
        let bytes = to_bytes(body, usize::MAX).await.unwrap();
        (parts, bytes)
    }

    fn header(parts: &Parts, name: &'static str) -> String {
        parts
            .headers
            .get(name)
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
    }

    // -- leaf bodies --

    #[tokio::test]
    async fn str_is_text_plain() {
        let (parts, body) = run("hi").await;
        assert_eq!(parts.status, StatusCode::OK);
        assert_eq!(header(&parts, "content-type"), "text/plain; charset=utf-8");
        assert_eq!(&body[..], b"hi");
    }

    #[tokio::test]
    async fn owned_and_borrowed_text_match() {
        for (parts, body) in [
            run(String::from("hi")).await,
            run(Box::<str>::from("hi")).await,
            run(Cow::Borrowed("hi")).await,
            run(Cow::<str>::Owned("hi".to_owned())).await,
        ] {
            assert_eq!(header(&parts, "content-type"), "text/plain; charset=utf-8");
            assert_eq!(&body[..], b"hi");
        }
    }

    #[tokio::test]
    async fn byte_bodies_are_octet_stream() {
        for (parts, body) in [
            run(b"hi".to_vec()).await,
            run(Bytes::from_static(b"hi")).await,
            run(BytesMut::from(&b"hi"[..])).await,
            run(*b"hi").await,
            run(b"hi").await,
            run(Box::<[u8]>::from(&b"hi"[..])).await,
        ] {
            assert_eq!(header(&parts, "content-type"), "application/octet-stream");
            assert_eq!(&body[..], b"hi");
        }
    }

    #[tokio::test]
    async fn unit_is_empty_ok() {
        let (parts, body) = run(()).await;
        assert_eq!(parts.status, StatusCode::OK);
        assert!(body.is_empty());
        assert!(parts.headers.is_empty());
    }

    #[tokio::test]
    async fn status_code_sets_status_with_empty_body() {
        let (parts, body) = run(StatusCode::NO_CONTENT).await;
        assert_eq!(parts.status, StatusCode::NO_CONTENT);
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn header_map_applies_headers() {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("x-test"),
            HeaderValue::from_static("1"),
        );
        let (parts, body) = run(headers).await;
        assert_eq!(header(&parts, "x-test"), "1");
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn extensions_are_applied() {
        #[derive(Clone, Debug, PartialEq)]
        struct Marker(u32);

        let mut extensions = Extensions::new();
        extensions.insert(Marker(7));
        let (parts, _) = run(extensions).await;
        assert_eq!(parts.extensions.get::<Marker>(), Some(&Marker(7)));
    }

    #[tokio::test]
    async fn response_passes_through_with_rebodied_stream() {
        let response = http::Response::builder()
            .status(StatusCode::ACCEPTED)
            .header("x-test", "1")
            .body(Full::new(Bytes::from_static(b"yo")))
            .unwrap();
        let (parts, body) = run(response).await;
        assert_eq!(parts.status, StatusCode::ACCEPTED);
        assert_eq!(header(&parts, "x-test"), "1");
        assert_eq!(&body[..], b"yo");
    }

    // -- header arrays --

    #[tokio::test]
    async fn header_array_is_into_response() {
        let (parts, body) = run([("x-test", "1"), ("x-other", "2")]).await;
        assert_eq!(header(&parts, "x-test"), "1");
        assert_eq!(header(&parts, "x-other"), "2");
        assert!(body.is_empty());
    }

    #[tokio::test]
    async fn invalid_header_name_is_an_error() {
        // A space is not allowed in a header name.
        let cx = Cx::default();
        assert!([("inva lid", "1")].into_response(&cx).await.is_err());
    }

    // -- tuples --

    #[tokio::test]
    async fn one_tuple_is_just_the_body() {
        let (parts, body) = run(("hi",)).await;
        assert_eq!(parts.status, StatusCode::OK);
        assert_eq!(&body[..], b"hi");
    }

    #[tokio::test]
    async fn status_then_body() {
        let (parts, body) = run((StatusCode::CREATED, "hi")).await;
        assert_eq!(parts.status, StatusCode::CREATED);
        assert_eq!(header(&parts, "content-type"), "text/plain; charset=utf-8");
        assert_eq!(&body[..], b"hi");
    }

    #[tokio::test]
    async fn headers_then_body() {
        let (parts, body) = run(([("x-test", "1")], "hi")).await;
        assert_eq!(header(&parts, "x-test"), "1");
        assert_eq!(&body[..], b"hi");
    }

    #[tokio::test]
    async fn status_headers_and_body() {
        let (parts, body) = run((StatusCode::CREATED, [("x-test", "1")], "hi")).await;
        assert_eq!(parts.status, StatusCode::CREATED);
        assert_eq!(header(&parts, "x-test"), "1");
        assert_eq!(&body[..], b"hi");
    }

    #[tokio::test]
    async fn multiple_response_parts_are_all_applied() {
        let (parts, body) = run((
            StatusCode::CREATED,
            [("x-one", "1")],
            [("x-two", "2")],
            "hi",
        )).await;
        assert_eq!(parts.status, StatusCode::CREATED);
        assert_eq!(header(&parts, "x-one"), "1");
        assert_eq!(header(&parts, "x-two"), "2");
        assert_eq!(&body[..], b"hi");
    }

    #[tokio::test]
    async fn optional_parts_are_applied_when_present() {
        let (present, _) = run((Some([("x-test", "1")]), "hi")).await;
        assert_eq!(header(&present, "x-test"), "1");

        let (absent, body) = run((Option::<[(&str, &str); 1]>::None, "hi")).await;
        assert!(absent.headers.get("x-test").is_none());
        assert_eq!(&body[..], b"hi");
    }

    #[tokio::test]
    async fn parts_template_seeds_status_and_headers() {
        let (mut template, _) = Response::new(Body::empty()).into_parts();
        template.status = StatusCode::IM_A_TEAPOT;
        template.headers.insert(
            HeaderName::from_static("x-test"),
            HeaderValue::from_static("1"),
        );

        let (parts, body) = run((template, "hi")).await;
        assert_eq!(parts.status, StatusCode::IM_A_TEAPOT);
        assert_eq!(header(&parts, "x-test"), "1");
        assert_eq!(&body[..], b"hi");
    }

    #[tokio::test]
    async fn response_unit_template_seeds_status_and_headers() {
        let template = http::Response::builder()
            .status(StatusCode::IM_A_TEAPOT)
            .header("x-test", "1")
            .body(())
            .unwrap();

        let (parts, body) = run((template, "hi")).await;
        assert_eq!(parts.status, StatusCode::IM_A_TEAPOT);
        assert_eq!(header(&parts, "x-test"), "1");
        assert_eq!(&body[..], b"hi");
    }
}
