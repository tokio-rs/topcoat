use std::fmt::{self, Display};
use std::ops::Deref;
use std::str::Utf8Error;

use bytes::Bytes;
use tokio_tungstenite::tungstenite;

/// A WebSocket message, received from or sent to the client.
///
/// `Text` and `Binary` are the messages an application exchanges. `Ping` and
/// `Pong` are the protocol's keep-alive probes: an incoming `Ping` is answered
/// automatically, so most applications only observe them. `Close` starts (or
/// acknowledges) the closing handshake, optionally carrying a [`CloseFrame`]
/// with a [code](CloseCode) and a reason.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub enum Message {
    /// A text message, guaranteed to be valid UTF-8.
    Text(Utf8Bytes),
    /// A binary message.
    Binary(Bytes),
    /// A ping probe, answered with a `Pong` automatically.
    Ping(Bytes),
    /// The answer to a ping probe.
    Pong(Bytes),
    /// A closing handshake message.
    Close(Option<CloseFrame>),
}

impl Message {
    /// Builds a text message.
    pub fn text(text: impl Into<Utf8Bytes>) -> Self {
        Self::Text(text.into())
    }

    /// Builds a binary message.
    pub fn binary(data: impl Into<Bytes>) -> Self {
        Self::Binary(data.into())
    }

    /// Consumes the message, returning its payload as bytes: the text or
    /// binary data, a probe's payload, or a close message's reason.
    #[must_use]
    pub fn into_data(self) -> Bytes {
        match self {
            Self::Text(text) => text.into(),
            Self::Binary(data) | Self::Ping(data) | Self::Pong(data) => data,
            Self::Close(frame) => frame.map_or_else(Bytes::new, |frame| frame.reason.into()),
        }
    }

    /// Consumes the message, returning its payload as UTF-8 text.
    ///
    /// # Errors
    ///
    /// Returns an error if the payload is not valid UTF-8. A `Text` message
    /// never fails; the other variants carry arbitrary bytes.
    pub fn into_text(self) -> Result<Utf8Bytes, Utf8Error> {
        match self {
            Self::Text(text) => Ok(text),
            message => message.into_data().try_into(),
        }
    }

    /// Converts to the message type the protocol implementation speaks.
    pub(crate) fn into_tungstenite(self) -> tungstenite::Message {
        match self {
            Self::Text(text) => tungstenite::Message::Text(text.0),
            Self::Binary(data) => tungstenite::Message::Binary(data),
            Self::Ping(data) => tungstenite::Message::Ping(data),
            Self::Pong(data) => tungstenite::Message::Pong(data),
            Self::Close(frame) => {
                tungstenite::Message::Close(frame.map(|frame| tungstenite::protocol::CloseFrame {
                    code: frame.code.into(),
                    reason: frame.reason.0,
                }))
            }
        }
    }

    /// Converts from the message type the protocol implementation speaks, or
    /// [`None`] for a raw frame, which is an implementation detail of writes
    /// and never surfaced to applications.
    pub(crate) fn from_tungstenite(message: tungstenite::Message) -> Option<Self> {
        match message {
            tungstenite::Message::Text(text) => Some(Self::Text(Utf8Bytes(text))),
            tungstenite::Message::Binary(data) => Some(Self::Binary(data)),
            tungstenite::Message::Ping(data) => Some(Self::Ping(data)),
            tungstenite::Message::Pong(data) => Some(Self::Pong(data)),
            tungstenite::Message::Close(frame) => {
                Some(Self::Close(frame.map(|frame| CloseFrame {
                    code: frame.code.into(),
                    reason: Utf8Bytes(frame.reason),
                })))
            }
            tungstenite::Message::Frame(_) => None,
        }
    }
}

impl From<String> for Message {
    fn from(text: String) -> Self {
        Self::Text(text.into())
    }
}

impl From<&str> for Message {
    fn from(text: &str) -> Self {
        Self::Text(text.into())
    }
}

impl From<Vec<u8>> for Message {
    fn from(data: Vec<u8>) -> Self {
        Self::Binary(data.into())
    }
}

impl From<Bytes> for Message {
    fn from(data: Bytes) -> Self {
        Self::Binary(data)
    }
}

/// Cheaply cloneable bytes guaranteed to contain valid UTF-8, used as the
/// payload of a text [`Message`].
///
/// Dereferences to [`str`], so string methods are available directly. Build
/// one from a string with [`From`], or from bytes with [`TryFrom`], which
/// validates the encoding.
#[derive(Debug, Clone, Default)]
pub struct Utf8Bytes(tungstenite::Utf8Bytes);

impl Utf8Bytes {
    /// Returns the text as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl Deref for Utf8Bytes {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for Utf8Bytes {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Display for Utf8Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Display::fmt(self.as_str(), f)
    }
}

impl PartialEq for Utf8Bytes {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}

impl Eq for Utf8Bytes {}

impl PartialEq<str> for Utf8Bytes {
    fn eq(&self, other: &str) -> bool {
        self.as_str() == other
    }
}

impl PartialEq<&str> for Utf8Bytes {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}

impl PartialEq<String> for Utf8Bytes {
    fn eq(&self, other: &String) -> bool {
        self.as_str() == other
    }
}

impl From<String> for Utf8Bytes {
    fn from(text: String) -> Self {
        Self(text.into())
    }
}

impl From<&str> for Utf8Bytes {
    fn from(text: &str) -> Self {
        Self(text.into())
    }
}

impl TryFrom<Bytes> for Utf8Bytes {
    type Error = Utf8Error;

    fn try_from(bytes: Bytes) -> Result<Self, Self::Error> {
        Ok(Self(bytes.try_into()?))
    }
}

impl TryFrom<Vec<u8>> for Utf8Bytes {
    type Error = Utf8Error;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        Ok(Self(bytes.try_into()?))
    }
}

impl From<Utf8Bytes> for Bytes {
    fn from(text: Utf8Bytes) -> Self {
        text.0.into()
    }
}

/// The payload of a close [`Message`]: a status code and a human-readable
/// reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CloseFrame {
    /// The status code explaining why the connection is closing, typically one
    /// of the [`close_code`] constants.
    pub code: CloseCode,
    /// A human-readable explanation, which may be empty.
    pub reason: Utf8Bytes,
}

/// The status code of a [`CloseFrame`].
///
/// The registered codes are available as the [`close_code`] constants; ranges
/// outside them are reserved for applications and extensions by the protocol.
pub type CloseCode = u16;

/// The registered [`CloseCode`] values a close message can carry, from RFC
/// 6455.
pub mod close_code {
    use super::CloseCode;

    /// The purpose for which the connection was established is fulfilled.
    pub const NORMAL: CloseCode = 1000;
    /// The endpoint is going away: the server shuts down, or the browser
    /// leaves the page.
    pub const AWAY: CloseCode = 1001;
    /// The endpoint received a message that violates the protocol.
    pub const PROTOCOL: CloseCode = 1002;
    /// The endpoint received a message type it cannot accept.
    pub const UNSUPPORTED: CloseCode = 1003;
    /// The endpoint received a message with inconsistent data, like non-UTF-8
    /// bytes in a text message.
    pub const INVALID: CloseCode = 1007;
    /// The endpoint received a message that violates its policy and no more
    /// specific code applies.
    pub const POLICY: CloseCode = 1008;
    /// The endpoint received a message too big for it to process.
    pub const SIZE: CloseCode = 1009;
    /// The client expected an extension the server did not negotiate.
    pub const EXTENSION: CloseCode = 1010;
    /// The server encountered an unexpected condition that prevents it from
    /// fulfilling the request.
    pub const ERROR: CloseCode = 1011;
    /// The server is restarting; the client may reconnect.
    pub const RESTART: CloseCode = 1012;
    /// The server is overloaded; the client should reconnect later or to a
    /// different server.
    pub const AGAIN: CloseCode = 1013;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_message_converts_from_strings() {
        assert_eq!(Message::text("hi"), Message::from("hi"));
        assert_eq!(Message::text("hi"), Message::from(String::from("hi")));
        assert_eq!(Message::text("hi"), Message::Text("hi".into()));
    }

    #[test]
    fn binary_message_converts_from_bytes() {
        assert_eq!(Message::binary(vec![1, 2]), Message::from(vec![1, 2]));
        assert_eq!(
            Message::binary(vec![1, 2]),
            Message::from(Bytes::from_static(&[1, 2]))
        );
    }

    #[test]
    fn into_data_returns_the_payload() {
        assert_eq!(Message::text("hi").into_data(), Bytes::from_static(b"hi"));
        assert_eq!(
            Message::binary(vec![1, 2]).into_data(),
            Bytes::from_static(&[1, 2])
        );
        assert_eq!(Message::Close(None).into_data(), Bytes::new());
        assert_eq!(
            Message::Close(Some(CloseFrame {
                code: close_code::NORMAL,
                reason: "done".into(),
            }))
            .into_data(),
            Bytes::from_static(b"done")
        );
    }

    #[test]
    fn into_text_validates_utf8() {
        assert_eq!(Message::text("hi").into_text().unwrap(), "hi");
        assert_eq!(Message::binary(b"hi".to_vec()).into_text().unwrap(), "hi");
        assert!(Message::binary(vec![0xff]).into_text().is_err());
    }

    #[test]
    fn utf8_bytes_rejects_invalid_utf8() {
        assert!(Utf8Bytes::try_from(vec![0xff]).is_err());
        assert!(Utf8Bytes::try_from(Bytes::from_static(&[0xff])).is_err());
        assert_eq!(Utf8Bytes::try_from(b"hi".to_vec()).unwrap(), "hi");
    }

    #[test]
    fn close_frame_roundtrips_through_tungstenite() {
        let close = Message::Close(Some(CloseFrame {
            code: close_code::AWAY,
            reason: "bye".into(),
        }));
        let roundtripped = Message::from_tungstenite(close.clone().into_tungstenite());
        assert_eq!(roundtripped, Some(close));
    }

    #[test]
    fn raw_frames_are_not_surfaced() {
        let frame = tungstenite::Message::Frame(tungstenite::protocol::frame::Frame::pong(
            Bytes::from_static(b"hi"),
        ));
        assert_eq!(Message::from_tungstenite(frame), None);
    }
}
