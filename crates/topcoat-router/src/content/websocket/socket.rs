use std::future::poll_fn;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures_core::Stream;
use futures_sink::Sink;
use http::HeaderValue;
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use tokio_tungstenite::{WebSocketStream, tungstenite};
use topcoat_core::error::{Error, Result};

use crate::content::websocket::Message;

/// A WebSocket connection to a client, obtained from
/// [`WebSocketUpgrade::on_upgrade`](crate::content::websocket::WebSocketUpgrade::on_upgrade).
///
/// Exchange [`Message`]s with [`recv`](Self::recv) and [`send`](Self::send),
/// and end the conversation with [`close`](Self::close). The connection also
/// implements [`Stream`] and [`Sink`], so the halves can be split and combined
/// with the usual stream and sink adapters.
#[must_use]
pub struct WebSocket {
    inner: WebSocketStream<TokioIo<Upgraded>>,
    protocol: Option<HeaderValue>,
}

impl WebSocket {
    pub(crate) fn new(
        inner: WebSocketStream<TokioIo<Upgraded>>,
        protocol: Option<HeaderValue>,
    ) -> Self {
        Self { inner, protocol }
    }

    /// Receives the next message, or [`None`] once the connection has closed.
    ///
    /// An incoming ping is answered automatically, but still surfaced as a
    /// [`Message::Ping`]. A returned error is fatal: the connection is broken,
    /// and subsequent calls return [`None`].
    pub async fn recv(&mut self) -> Option<Result<Message>> {
        poll_fn(|cx| Pin::new(&mut *self).poll_next(cx)).await
    }

    /// Sends a message and flushes it to the client.
    ///
    /// # Errors
    ///
    /// Returns an error if the message cannot be written, for example because
    /// the client disconnected or the message exceeds the configured sizes.
    pub async fn send(&mut self, message: Message) -> Result<()> {
        poll_fn(|cx| Pin::new(&mut *self).poll_ready(cx)).await?;
        Pin::new(&mut *self).start_send(message)?;
        poll_fn(|cx| Pin::new(&mut *self).poll_flush(cx)).await
    }

    /// Performs the closing handshake and consumes the connection.
    ///
    /// To close with a status code and reason instead, [`send`](Self::send) a
    /// [`Message::Close`] carrying a
    /// [`CloseFrame`](crate::content::websocket::CloseFrame) first.
    ///
    /// # Errors
    ///
    /// Returns an error if the handshake cannot be written to the client.
    pub async fn close(mut self) -> Result<()> {
        poll_fn(|cx| Pin::new(&mut self).poll_close(cx)).await
    }

    /// Returns the subprotocol negotiated during the handshake, if any.
    #[must_use]
    pub fn protocol(&self) -> Option<&HeaderValue> {
        self.protocol.as_ref()
    }
}

impl Stream for WebSocket {
    type Item = Result<Message>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            let message = match ready!(Pin::new(&mut self.inner).poll_next(cx)) {
                Some(Ok(message)) => message,
                // A closed connection ends the stream instead of erroring, so
                // receive loops terminate cleanly.
                Some(Err(
                    tungstenite::Error::ConnectionClosed | tungstenite::Error::AlreadyClosed,
                ))
                | None => return Poll::Ready(None),
                Some(Err(error)) => return Poll::Ready(Some(Err(error.into()))),
            };

            // Raw frames are a write-side implementation detail; skip them.
            if let Some(message) = Message::from_tungstenite(message) {
                return Poll::Ready(Some(Ok(message)));
            }
        }
    }
}

impl Sink<Message> for WebSocket {
    type Error = Error;

    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.inner).poll_ready(cx).map_err(Into::into)
    }

    fn start_send(mut self: Pin<&mut Self>, message: Message) -> Result<()> {
        Pin::new(&mut self.inner)
            .start_send(message.into_tungstenite())
            .map_err(Into::into)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.inner).poll_flush(cx).map_err(Into::into)
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<()>> {
        Pin::new(&mut self.inner).poll_close(cx).map_err(Into::into)
    }
}
