use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_util::Stream;
use http::HeaderMap;
use topcoat_core::runtime::{
    context::Cx,
    error::{Error, Result},
};

use crate::runtime::{
    Body, FromRequest, OptionalFromRequest, bad_request, content_type, internal_server_error,
};

/// `multipart/form-data` request extractor, commonly used for file uploads.
///
/// # Examples
///
/// ```rust,ignore
/// use topcoat::{
///     Result,
///     router::{Multipart, route},
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
    async fn from_request(cx: &Cx, body: Body) -> Result<Self> {
        let boundary = content_type(cx)
            .and_then(|content_type| multer::parse_boundary(content_type).ok())
            .ok_or_else(invalid_boundary)?;

        Ok(Self {
            inner: multer::Multipart::new(body.into_data_stream(), boundary),
        })
    }
}

impl OptionalFromRequest for Multipart {
    async fn from_request(cx: &Cx, body: Body) -> Result<Option<Self>> {
        let Some(content_type) = content_type(cx) else {
            return Ok(None);
        };

        match multer::parse_boundary(content_type) {
            Ok(boundary) => Ok(Some(Self {
                inner: multer::Multipart::new(body.into_data_stream(), boundary),
            })),
            Err(multer::Error::NoMultipart) => Ok(None),
            Err(_) => Err(invalid_boundary()),
        }
    }
}

impl Multipart {
    /// Yields the next [`Field`] if available.
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
        self.inner.content_type().map(|mime| mime.as_ref())
    }

    /// The headers of the field as a [`HeaderMap`].
    #[must_use]
    pub fn headers(&self) -> &HeaderMap {
        self.inner.headers()
    }

    /// Reads the full data of the field into [`Bytes`].
    pub async fn bytes(self) -> Result<Bytes> {
        self.inner.bytes().await.map_err(multipart_error)
    }

    /// Reads the full data of the field as text.
    pub async fn text(self) -> Result<String> {
        self.inner.text().await.map_err(multipart_error)
    }

    /// Streams a chunk of the field data, returning [`None`] once exhausted.
    ///
    /// This does the same thing as the [`Stream`] implementation.
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

fn invalid_boundary() -> Error {
    bad_request("invalid `boundary` for `multipart/form-data` request").into()
}

fn multipart_error(error: multer::Error) -> Error {
    if is_client_error(&error) {
        bad_request(error.to_string()).into()
    } else {
        internal_server_error(error).into()
    }
}

/// Classifies a `multer` error as a client-side (`400`) error, mirroring the
/// status codes that `axum` reports for the same conditions.
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
