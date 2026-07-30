use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use http_body::{Frame, SizeHint};
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::{BodyExt, BodyStream, Empty, Full, LengthLimitError, Limited};
use topcoat_core::error::Result;

use crate::error::{bad_request, content_too_large};

/// A boxed error type used by the response body machinery.
pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// The HTTP body type used for both requests and responses.
#[must_use]
pub struct Body(UnsyncBoxBody<Bytes, BoxError>);

impl Body {
    /// Creates an empty body.
    pub fn empty() -> Self {
        Self::from(Bytes::new())
    }

    /// Wraps any [`http_body::Body`] that yields [`Bytes`].
    pub fn new<B>(body: B) -> Self
    where
        B: http_body::Body<Data = Bytes> + Send + 'static,
        B::Error: Into<BoxError>,
    {
        Self(body.map_err(Into::into).boxed_unsync())
    }

    /// Consumes the body, returning a [`Stream`](futures_core::Stream) of its
    /// data frames.
    pub fn into_data_stream(self) -> BodyDataStream {
        BodyDataStream(BodyStream::new(self))
    }
}

impl Default for Body {
    fn default() -> Self {
        Self::empty()
    }
}

impl From<Bytes> for Body {
    fn from(bytes: Bytes) -> Self {
        Self(
            Full::new(bytes)
                .map_err(|never: Infallible| match never {})
                .boxed_unsync(),
        )
    }
}

impl From<()> for Body {
    fn from((): ()) -> Self {
        Self(
            Empty::new()
                .map_err(|never: Infallible| match never {})
                .boxed_unsync(),
        )
    }
}

impl From<Vec<u8>> for Body {
    fn from(value: Vec<u8>) -> Self {
        Self::from(Bytes::from(value))
    }
}

impl From<&'static [u8]> for Body {
    fn from(value: &'static [u8]) -> Self {
        Self::from(Bytes::from_static(value))
    }
}

impl From<String> for Body {
    fn from(value: String) -> Self {
        Self::from(Bytes::from(value))
    }
}

impl From<&'static str> for Body {
    fn from(value: &'static str) -> Self {
        Self::from(Bytes::from_static(value.as_bytes()))
    }
}

impl http_body::Body for Body {
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        Pin::new(&mut self.0).poll_frame(cx)
    }

    fn size_hint(&self) -> SizeHint {
        self.0.size_hint()
    }

    fn is_end_stream(&self) -> bool {
        self.0.is_end_stream()
    }
}

impl std::fmt::Debug for Body {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Body").finish_non_exhaustive()
    }
}

/// A [`Stream`](futures_core::Stream) over the data frames of a [`Body`],
/// yielding the raw [`Bytes`] of each frame.
#[must_use]
pub struct BodyDataStream(BodyStream<Body>);

impl futures_core::Stream for BodyDataStream {
    type Item = Result<Bytes, BoxError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            return match Pin::new(&mut self.0).poll_next(cx) {
                Poll::Ready(Some(Ok(frame))) => match frame.into_data() {
                    Ok(data) => Poll::Ready(Some(Ok(data))),
                    Err(_frame) => continue,
                },
                Poll::Ready(Some(Err(error))) => Poll::Ready(Some(Err(error))),
                Poll::Ready(None) => Poll::Ready(None),
                Poll::Pending => Poll::Pending,
            };
        }
    }
}

/// Collects an entire [`Body`] into [`Bytes`], failing if it exceeds `limit`.
///
/// Pass [`usize::MAX`] to read the body without enforcing a limit. When
/// reading a request body, pass the request's
/// [`body_limit`](crate::body_limit) so the configured limit applies.
///
/// # Errors
///
/// Returns a [`ContentTooLargeError`](crate::error::ContentTooLargeError) when
/// the body exceeds `limit` bytes, and a
/// [`BadRequestError`](crate::error::BadRequestError) when reading it fails.
/// Both render themselves as a response, so a
/// [`FromRequest`](crate::FromRequest) implementation can propagate them with
/// `?` rather than mapping them by hand.
pub async fn to_bytes(body: Body, limit: usize) -> Result<Bytes> {
    let collected = if limit == usize::MAX {
        body.collect().await
    } else {
        Limited::new(body, limit).collect().await
    };

    match collected {
        Ok(collected) => Ok(collected.to_bytes()),
        Err(error) => Err(match error.downcast::<LengthLimitError>() {
            Ok(_) => content_too_large().into(),
            Err(error) => bad_request(format!("failed to read the body: {error}")).into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{BadRequestError, ContentTooLargeError};

    /// A body that fails on its first frame, standing in for a connection that
    /// breaks mid-stream.
    struct FailingBody;

    impl http_body::Body for FailingBody {
        type Data = Bytes;
        type Error = BoxError;

        fn poll_frame(
            self: Pin<&mut Self>,
            _cx: &mut Context<'_>,
        ) -> Poll<Option<Result<Frame<Bytes>, BoxError>>> {
            Poll::Ready(Some(Err("body stream broke".into())))
        }
    }

    #[tokio::test]
    async fn a_body_within_the_limit_reads_in_full() {
        let bytes = to_bytes(Body::from("hello"), 1024).await.unwrap();
        assert_eq!(bytes, Bytes::from_static(b"hello"));
    }

    #[tokio::test]
    async fn usize_max_reads_without_enforcing_a_limit() {
        let bytes = to_bytes(Body::from("hello"), usize::MAX).await.unwrap();
        assert_eq!(bytes, Bytes::from_static(b"hello"));
    }

    #[tokio::test]
    async fn a_body_over_the_limit_is_content_too_large() {
        let error = to_bytes(Body::from("hello"), 4).await.unwrap_err();
        assert!(error.is::<ContentTooLargeError>());
    }

    #[tokio::test]
    async fn a_read_failure_is_a_bad_request_carrying_the_cause() {
        // Both branches are covered: a limit installs `Limited` around the
        // body, which boxes the failure a second time.
        for limit in [usize::MAX, 1024] {
            let error = to_bytes(Body::new(FailingBody), limit).await.unwrap_err();
            let error = error
                .downcast::<BadRequestError>()
                .expect("a read failure is a bad request");
            assert_eq!(
                error.description(),
                "failed to read the body: body stream broke"
            );
        }
    }
}
