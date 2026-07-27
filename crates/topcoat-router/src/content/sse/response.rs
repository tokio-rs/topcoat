use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_core::Stream;
use http::header::{CACHE_CONTROL, CONTENT_TYPE, HeaderValue};
use http_body::Frame;
use topcoat_core::{
    context::Cx,
    error::{Error, Result},
};

use crate::content::sse::{Event, KeepAlive, KeepAliveTimer};
use crate::{Body, BoxError, IntoResponse, Response};

/// Server-sent events response: streams [`Event`]s to the client over a
/// long-lived connection.
///
/// Wrap a [`Stream`] of events to reply with `Content-Type: text/event-stream`
/// and send each event as the stream yields it. The connection stays open
/// until the stream ends, an `Err` item occurs, or the client disconnects;
/// a disconnect drops the stream, so cleanup belongs in the stream's `Drop`.
/// The router never compresses event streams, so events are not delayed by
/// an encoder buffer.
///
/// A browser `EventSource` reconnects automatically when the connection is
/// lost. Send [`Event::retry`] to tune its reconnection delay, and
/// [`Event::id`] together with [`last_event_id`](crate::content::sse::last_event_id)
/// to resume a stream where the client left off.
///
/// # Examples
///
/// ```rust
/// use futures_core::Stream;
/// use topcoat::{
///     Result,
///     router::{
///         content::sse::{Event, KeepAlive, Sse},
///         route,
///     },
/// };
///
/// #[route(GET "/events")]
/// async fn events() -> Result<Sse<impl Stream<Item = Result<Event>> + use<>>> {
///     let events = futures_util::stream::iter(
///         ["one", "two", "three"].map(|name| Ok(Event::new().data(name))),
///     );
///     Ok(Sse::new(events).keep_alive(KeepAlive::new()))
/// }
/// ```
///
/// The `use<>` bound keeps the stream from capturing the request context,
/// which a route's response must not borrow.
#[must_use]
pub struct Sse<S> {
    stream: S,
    keep_alive: Option<KeepAlive>,
}

impl<S> Sse<S> {
    /// Creates a response streaming the events yielded by `stream`.
    pub fn new(stream: S) -> Self {
        Self {
            stream,
            keep_alive: None,
        }
    }

    /// Sends keep-alive events whenever the stream is idle, so proxies and
    /// clients do not drop a quiet connection. No keep-alive by default.
    pub fn keep_alive(mut self, keep_alive: KeepAlive) -> Self {
        self.keep_alive = Some(keep_alive);
        self
    }
}

impl<S> fmt::Debug for Sse<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sse")
            .field("keep_alive", &self.keep_alive)
            .finish_non_exhaustive()
    }
}

impl<S, E> IntoResponse for Sse<S>
where
    S: Stream<Item = Result<Event, E>> + Send + 'static,
    E: Into<Error>,
{
    fn into_response(self, cx: &Cx) -> Result<Response> {
        let keep_alive = self.keep_alive.map(KeepAlive::into_timer).transpose()?;
        let body = Body::new(SseBody {
            stream: Box::pin(self.stream),
            keep_alive,
        });
        (
            [
                (CONTENT_TYPE, HeaderValue::from_static("text/event-stream")),
                (CACHE_CONTROL, HeaderValue::from_static("no-cache")),
            ],
            body,
        )
            .into_response(cx)
    }
}

/// The body of an [`Sse`] response, serializing each event of the stream into
/// a data frame and filling idle gaps with keep-alive events.
struct SseBody<S> {
    stream: Pin<Box<S>>,
    keep_alive: Option<KeepAliveTimer>,
}

impl<S, E> http_body::Body for SseBody<S>
where
    S: Stream<Item = Result<Event, E>>,
    E: Into<Error>,
{
    type Data = Bytes;
    type Error = BoxError;

    fn poll_frame(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, BoxError>>> {
        let this = self.get_mut();
        match this.stream.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(event))) => {
                let frame = match event.serialize() {
                    Ok(frame) => frame,
                    Err(error) => return Poll::Ready(Some(Err(error.into()))),
                };
                if let Some(keep_alive) = &mut this.keep_alive {
                    keep_alive.defer();
                }
                Poll::Ready(Some(Ok(Frame::data(frame))))
            }
            Poll::Ready(Some(Err(error))) => {
                let error: Error = error.into();
                Poll::Ready(Some(Err(error.into())))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => match &mut this.keep_alive {
                Some(keep_alive) => keep_alive
                    .poll_frame(cx)
                    .map(|frame| Some(Ok(Frame::data(frame)))),
                None => Poll::Pending,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures_util::StreamExt;
    use futures_util::stream;
    use tokio::time::Instant;

    use super::*;
    use crate::to_bytes;

    #[tokio::test]
    async fn the_response_streams_events_with_event_stream_headers() {
        let events = stream::iter([
            Ok::<_, Error>(Event::new().data("one")),
            Ok(Event::new().data("two")),
        ]);
        let response = Sse::new(events).into_response(&Cx::default()).unwrap();

        assert_eq!(
            response.headers().get(CONTENT_TYPE).unwrap(),
            "text/event-stream"
        );
        assert_eq!(response.headers().get(CACHE_CONTROL).unwrap(), "no-cache");

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"data: one\n\ndata: two\n\n");
    }

    #[tokio::test]
    async fn a_stream_error_ends_the_body() {
        let events = stream::iter([
            Ok(Event::new().data("one")),
            Err(Error::from(std::io::Error::other("source failed"))),
        ]);
        let response = Sse::new(events).into_response(&Cx::default()).unwrap();

        let mut frames = response.into_body().into_data_stream();
        let frame = frames.next().await.unwrap().unwrap();
        assert_eq!(&frame[..], b"data: one\n\n");
        let error = frames.next().await.unwrap().unwrap_err();
        assert_eq!(error.to_string(), "source failed");
        assert!(frames.next().await.is_none());
    }

    #[tokio::test(start_paused = true)]
    async fn keep_alives_fill_idle_gaps() {
        let response = Sse::new(stream::pending::<Result<Event>>())
            .keep_alive(
                KeepAlive::new()
                    .interval(Duration::from_secs(10))
                    .text("ping"),
            )
            .into_response(&Cx::default())
            .unwrap();

        let started = Instant::now();
        let mut frames = response.into_body().into_data_stream();
        for seconds in [10, 20] {
            let frame = frames.next().await.unwrap().unwrap();
            assert_eq!(&frame[..], b": ping\n\n");
            assert_eq!(started.elapsed(), Duration::from_secs(seconds));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn events_defer_the_keep_alive() {
        let events = stream::once(async {
            tokio::time::sleep(Duration::from_secs(6)).await;
            Ok::<_, Error>(Event::new().data("hi"))
        })
        .chain(stream::pending());
        let response = Sse::new(events)
            .keep_alive(KeepAlive::new().interval(Duration::from_secs(10)))
            .into_response(&Cx::default())
            .unwrap();

        let started = Instant::now();
        let mut frames = response.into_body().into_data_stream();

        let frame = frames.next().await.unwrap().unwrap();
        assert_eq!(&frame[..], b"data: hi\n\n");
        assert_eq!(started.elapsed(), Duration::from_secs(6));

        // The keep-alive runs 10 seconds after the event, not 10 seconds
        // after the stream started.
        let frame = frames.next().await.unwrap().unwrap();
        assert_eq!(&frame[..], b": \n\n");
        assert_eq!(started.elapsed(), Duration::from_secs(16));
    }
}
