#![doc = include_str!("../../docs/content/multipart.md")]

use std::{
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_util::Stream;
use http::HeaderMap;
use http_body_util::{LengthLimitError, Limited};
use topcoat_core::{
    context::Cx,
    error::{Error, Result},
};

use crate::{
    Body, body_limit,
    error::{bad_request, content_too_large, internal_server_error},
    request::{FromRequest, OptionalFromRequest, content_type},
};

/// `multipart/form-data` request extractor, commonly used for file uploads.
///
/// Iterate the request's parts with
/// [`next_field`](Multipart::next_field); each [`Field`] exposes its metadata
/// ([`name`](Field::name), [`file_name`](Field::file_name),
/// [`content_type`](Field::content_type), [`headers`](Field::headers)) and its
/// data ([`bytes`](Field::bytes), [`text`](Field::text),
/// [`chunk`](Field::chunk)). A [`Field`] also implements [`Stream`], so its
/// chunks can be consumed with the usual stream combinators.
///
/// Wrap it in [`Option`] to make the body optional: the extractor yields
/// [`None`] when the request carries no `multipart/form-data` body.
///
/// The stream reads at most the request's body limit across all fields;
/// register a [`BodyLimit`](crate::BodyLimit) layer to raise it for an
/// upload route.
///
/// # Examples
///
/// ```rust
/// use topcoat::{
///     Result,
///     router::{content::multipart::Multipart, route},
/// };
///
/// #[route(POST "/api/upload")]
/// async fn upload(mut multipart: Multipart) -> Result<&'static str> {
///     while let Some(field) = multipart.next_field().await? {
///         let name = field.name().map(str::to_owned);
///         let data = field.bytes().await?;
///
///         println!("field `{name:?}` is {} bytes", data.len());
///     }
///
///     Ok("received")
/// }
/// ```
#[derive(Debug)]
#[must_use]
pub struct Multipart {
    inner: multer::Multipart<'static>,
}

impl FromRequest for Multipart {
    fn from_request(cx: &Cx, body: Body) -> impl Future<Output = Result<Self>> {
        core::future::ready(
            content_type(cx)
                .and_then(|content_type| multer::parse_boundary(content_type).ok())
                .map(|boundary| Self {
                    inner: multer::Multipart::new(limited(cx, body).into_data_stream(), boundary),
                })
                .ok_or_else(invalid_boundary),
        )
    }
}

impl OptionalFromRequest for Multipart {
    fn from_request(cx: &Cx, body: Body) -> impl Future<Output = Result<Option<Self>>> {
        let Some(content_type) = content_type(cx) else {
            return std::future::ready(Ok(None));
        };

        std::future::ready(match multer::parse_boundary(content_type) {
            Ok(boundary) => Ok(Some(Self {
                inner: multer::Multipart::new(limited(cx, body).into_data_stream(), boundary),
            })),
            Err(multer::Error::NoMultipart) => Ok(None),
            Err(_) => Err(invalid_boundary()),
        })
    }
}

impl Multipart {
    /// Yields the next [`Field`] if available.
    ///
    /// # Errors
    ///
    /// Returns an error if reading the next field from the multipart stream
    /// fails; a body over the request's body limit is classified as
    /// `413 Content Too Large`, malformed requests as `400 Bad Request`, and
    /// other failures as `500 Internal Server Error`.
    pub async fn next_field(&mut self) -> Result<Option<Field<'_>>> {
        let field = self.inner.next_field().await.map_err(multipart_error)?;

        Ok(field.map(|inner| Field {
            inner,
            _multipart: self,
        }))
    }
}

/// A single field in a multipart stream.
#[derive(Debug)]
#[must_use]
pub struct Field<'a> {
    inner: multer::Field<'static>,
    // `multer` requires there to only be one live `multer::Field` at any point.
    // Borrowing the `Multipart` mutably enforces that statically.
    _multipart: &'a mut Multipart,
}

impl Field<'_> {
    /// The field name found in the `Content-Disposition` header.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.inner.name()
    }

    /// The file name found in the `Content-Disposition` header.
    #[must_use]
    pub fn file_name(&self) -> Option<&str> {
        self.inner.file_name()
    }

    /// The `Content-Type` of the field, if present.
    #[must_use]
    pub fn content_type(&self) -> Option<&str> {
        self.inner.content_type().map(std::convert::AsRef::as_ref)
    }

    /// The headers of the field as a [`HeaderMap`].
    #[must_use]
    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }

    /// Reads the full data of the field into [`Bytes`].
    ///
    /// # Errors
    ///
    /// Returns an error if reading the field data fails; a body over the request's body
    /// limit is classified as `413 Content Too Large`, malformed requests as
    /// `400 Bad Request`, and other failures as `500 Internal Server Error`.
    pub async fn bytes(self) -> Result<Bytes> {
        self.inner.bytes().await.map_err(multipart_error)
    }

    /// Reads the full data of the field as text.
    ///
    /// # Errors
    ///
    /// Returns an error if reading the field text fails; a body over the request's body
    /// limit is classified as `413 Content Too Large`, malformed requests as
    /// `400 Bad Request`, and other failures as `500 Internal Server Error`.
    pub async fn text(self) -> Result<String> {
        self.inner.text().await.map_err(multipart_error)
    }

    /// Streams a chunk of the field data, returning [`None`] once exhausted.
    ///
    /// This does the same thing as the [`Stream`] implementation.
    ///
    /// # Errors
    ///
    /// Returns an error if reading the next chunk fails; a body over the request's body
    /// limit is classified as `413 Content Too Large`, malformed requests as
    /// `400 Bad Request`, and other failures as `500 Internal Server Error`.
    pub async fn chunk(&mut self) -> Result<Option<Bytes>> {
        self.inner.chunk().await.map_err(multipart_error)
    }
}

impl Stream for Field<'_> {
    type Item = Result<Bytes>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner)
            .poll_next(cx)
            .map_err(multipart_error)
    }
}

/// Caps `body` at the request's [`body_limit`], so reading past the limit
/// fails with an error classified as `413 Content Too Large`.
fn limited(cx: &Cx, body: Body) -> Body {
    Body::new(Limited::new(body, body_limit(cx)))
}

fn invalid_boundary() -> Error {
    bad_request("invalid `boundary` for `multipart/form-data` request").into()
}

fn multipart_error(error: multer::Error) -> Error {
    if is_length_limit_error(&error) {
        content_too_large().into()
    } else if is_client_error(&error) {
        bad_request(error.to_string()).into()
    } else {
        internal_server_error(error).into()
    }
}

/// Returns whether the multipart stream failed because the request body
/// exceeded the request's body limit.
fn is_length_limit_error(error: &multer::Error) -> bool {
    match error {
        multer::Error::StreamReadFailed(error) => error.is::<LengthLimitError>(),
        _ => false,
    }
}

/// Classifies a `multer` error as a client-side (`400`) error when the request
/// body or multipart headers are malformed.
fn is_client_error(error: &multer::Error) -> bool {
    match error {
        multer::Error::UnknownField { .. }
        | multer::Error::IncompleteFieldData { .. }
        | multer::Error::IncompleteHeaders
        | multer::Error::ReadHeaderFailed(..)
        | multer::Error::DecodeHeaderName { .. }
        | multer::Error::DecodeContentType(..)
        | multer::Error::NoBoundary
        | multer::Error::DecodeHeaderValue { .. }
        | multer::Error::NoMultipart
        | multer::Error::IncompleteStream
        | multer::Error::FieldSizeExceeded { .. }
        | multer::Error::StreamSizeExceeded { .. } => true,
        multer::Error::StreamReadFailed(error) => error
            .downcast_ref::<multer::Error>()
            .is_some_and(is_client_error),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use http::{Request, header::CONTENT_TYPE};
    use topcoat_core::context::{Cx, CxTestBuilder};

    use super::*;
    use crate::{
        Body,
        body_limit::DEFAULT_BODY_LIMIT,
        error::{BadRequestError, ContentTooLargeError},
        request::{FromRequest, OptionalFromRequest},
    };

    const BOUNDARY: &str = "X-TOPCOAT-BOUNDARY";

    /// The `Content-Type` header value for a `multipart/form-data` request using
    /// [`BOUNDARY`].
    fn multipart_content_type() -> String {
        format!("multipart/form-data; boundary={BOUNDARY}")
    }

    /// Builds a `Cx` carrying request `Parts` with the given `Content-Type`
    /// header, or no header at all when `content_type` is [`None`].
    fn cx_with_content_type(content_type: Option<&str>) -> Cx {
        let mut builder = Request::builder();
        if let Some(content_type) = content_type {
            builder = builder.header(CONTENT_TYPE, content_type);
        }

        let (parts, ()) = builder.body(()).expect("request should build").into_parts();

        CxTestBuilder::new().request_context(parts).build()
    }

    /// A two-field multipart body: a plain text `greeting` field and an
    /// `upload` file field with a filename and content type.
    fn sample_body() -> String {
        format!(
            "--{BOUNDARY}\r\n\
             Content-Disposition: form-data; name=\"greeting\"\r\n\
             \r\n\
             hello\r\n\
             --{BOUNDARY}\r\n\
             Content-Disposition: form-data; name=\"upload\"; filename=\"hello.txt\"\r\n\
             Content-Type: text/plain\r\n\
             \r\n\
             file body\r\n\
             --{BOUNDARY}--\r\n"
        )
    }

    #[tokio::test]
    async fn from_request_reads_fields_and_metadata() {
        let cx = cx_with_content_type(Some(&multipart_content_type()));
        let mut multipart =
            <Multipart as FromRequest>::from_request(&cx, Body::from(sample_body()))
                .await
                .expect("valid multipart request");

        let greeting = multipart
            .next_field()
            .await
            .expect("reading the first field succeeds")
            .expect("a first field is present");
        assert_eq!(greeting.name(), Some("greeting"));
        assert_eq!(greeting.file_name(), None);
        assert_eq!(greeting.content_type(), None);
        assert_eq!(greeting.text().await.expect("field text"), "hello");

        let upload = multipart
            .next_field()
            .await
            .expect("reading the second field succeeds")
            .expect("a second field is present");
        assert_eq!(upload.name(), Some("upload"));
        assert_eq!(upload.file_name(), Some("hello.txt"));
        assert_eq!(upload.content_type(), Some("text/plain"));
        assert_eq!(
            upload
                .headers()
                .get(CONTENT_TYPE)
                .map(http::HeaderValue::as_bytes),
            Some(b"text/plain".as_slice())
        );
        assert_eq!(
            &upload.bytes().await.expect("field bytes")[..],
            b"file body"
        );

        assert!(
            multipart
                .next_field()
                .await
                .expect("reading past the end succeeds")
                .is_none(),
            "the stream is exhausted after both fields"
        );
    }

    #[tokio::test]
    async fn field_chunk_streams_field_data() {
        let cx = cx_with_content_type(Some(&multipart_content_type()));
        let mut multipart =
            <Multipart as FromRequest>::from_request(&cx, Body::from(sample_body()))
                .await
                .expect("valid multipart request");

        let mut field = multipart
            .next_field()
            .await
            .expect("reading the first field succeeds")
            .expect("a first field is present");

        let mut data = Vec::new();
        while let Some(chunk) = field.chunk().await.expect("reading a chunk succeeds") {
            data.extend_from_slice(&chunk);
        }
        assert_eq!(data, b"hello");
    }

    #[tokio::test]
    async fn from_request_without_content_type_is_bad_request() {
        let cx = cx_with_content_type(None);
        let error = <Multipart as FromRequest>::from_request(&cx, Body::empty())
            .await
            .expect_err("missing content type is rejected");

        assert!(error.downcast_ref::<BadRequestError>().is_some());
    }

    #[tokio::test]
    async fn from_request_with_non_multipart_content_type_is_bad_request() {
        let cx = cx_with_content_type(Some("application/json"));
        let error = <Multipart as FromRequest>::from_request(&cx, Body::empty())
            .await
            .expect_err("a non-multipart content type is rejected");

        assert!(error.downcast_ref::<BadRequestError>().is_some());
    }

    #[tokio::test]
    async fn optional_from_request_without_content_type_is_none() {
        let cx = cx_with_content_type(None);
        let multipart = <Multipart as OptionalFromRequest>::from_request(&cx, Body::empty())
            .await
            .expect("an absent content type is not an error");

        assert!(multipart.is_none());
    }

    #[tokio::test]
    async fn optional_from_request_with_non_multipart_content_type_is_none() {
        let cx = cx_with_content_type(Some("application/json"));
        let multipart = <Multipart as OptionalFromRequest>::from_request(&cx, Body::empty())
            .await
            .expect("a non-multipart content type is not an error");

        assert!(multipart.is_none());
    }

    #[tokio::test]
    async fn optional_from_request_with_multipart_content_type_is_some() {
        let cx = cx_with_content_type(Some(&multipart_content_type()));
        let multipart =
            <Multipart as OptionalFromRequest>::from_request(&cx, Body::from(sample_body()))
                .await
                .expect("a valid multipart request is not an error");

        let mut multipart = multipart.expect("a multipart payload is present");
        let field = multipart
            .next_field()
            .await
            .expect("reading the first field succeeds")
            .expect("a first field is present");
        assert_eq!(field.name(), Some("greeting"));
    }

    #[tokio::test]
    async fn optional_from_request_with_multipart_without_boundary_is_bad_request() {
        let cx = cx_with_content_type(Some("multipart/form-data"));
        let error = <Multipart as OptionalFromRequest>::from_request(&cx, Body::empty())
            .await
            .expect_err("a multipart content type without a boundary is rejected");

        assert!(error.downcast_ref::<BadRequestError>().is_some());
    }

    #[tokio::test]
    async fn truncated_field_data_is_bad_request() {
        // The headers are complete, so `next_field` yields the field, but the
        // data is never terminated by a boundary, so reading it fails with a
        // client error that maps to a `400`.
        let body = format!(
            "--{BOUNDARY}\r\n\
             Content-Disposition: form-data; name=\"greeting\"\r\n\
             \r\n\
             hello"
        );
        let cx = cx_with_content_type(Some(&multipart_content_type()));
        let mut multipart = <Multipart as FromRequest>::from_request(&cx, Body::from(body))
            .await
            .expect("valid multipart request");

        let field = multipart
            .next_field()
            .await
            .expect("reading the first field succeeds")
            .expect("a first field is present");
        let error = field
            .bytes()
            .await
            .expect_err("reading truncated field data fails");

        assert!(error.downcast_ref::<BadRequestError>().is_some());
    }

    #[tokio::test]
    async fn a_body_over_the_limit_is_content_too_large() {
        let data = "x".repeat(DEFAULT_BODY_LIMIT + 1);
        let body = format!(
            "--{BOUNDARY}\r\n\
             Content-Disposition: form-data; name=\"upload\"\r\n\
             \r\n\
             {data}\r\n\
             --{BOUNDARY}--\r\n"
        );
        let cx = cx_with_content_type(Some(&multipart_content_type()));
        let mut multipart = <Multipart as FromRequest>::from_request(&cx, Body::from(body))
            .await
            .expect("valid multipart request");

        let error = multipart
            .next_field()
            .await
            .expect_err("a body over the limit is rejected");

        assert!(error.downcast_ref::<ContentTooLargeError>().is_some());
    }

    #[test]
    fn malformed_body_errors_are_classified_as_client_errors() {
        assert!(is_client_error(&multer::Error::NoMultipart));
        assert!(is_client_error(&multer::Error::IncompleteStream));
        assert!(is_client_error(&multer::Error::IncompleteHeaders));
    }
}
