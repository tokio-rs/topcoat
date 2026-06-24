use std::convert::Infallible;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use http_body::{Frame, SizeHint};
use http_body_util::combinators::UnsyncBoxBody;
use http_body_util::{BodyExt, BodyStream, Empty, Full, Limited};

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
/// Pass [`usize::MAX`] to read the body without enforcing a limit.
///
/// # Errors
///
/// Returns an error if reading the body fails or if it exceeds `limit` bytes.
pub async fn to_bytes(body: Body, limit: usize) -> Result<Bytes, BoxError> {
    if limit == usize::MAX {
        Ok(body.collect().await?.to_bytes())
    } else {
        Ok(Limited::new(body, limit).collect().await?.to_bytes())
    }
}
