use std::fmt;
use std::time::Duration;

use bytes::{Bytes, BytesMut};
use serde::Serialize;
use topcoat_core::{
    context::Cx,
    error::{Error, Result},
};

use crate::headers;

/// A server-sent event, assembled field by field.
///
/// Every builder method replaces the field it sets. An event without any
/// fields serializes to a blank line, which a client ignores.
///
/// # Examples
///
/// ```rust
/// use topcoat::{Result, router::content::sse::Event};
///
/// fn tick(count: u32) -> Result<Event> {
///     Event::new()
///         .event("tick")
///         .id(count.to_string())
///         .json_data(&count)
/// }
/// ```
#[derive(Clone, Debug, Default)]
#[must_use]
pub struct Event {
    comment: Option<String>,
    kind: Option<String>,
    data: Option<String>,
    id: Option<String>,
    retry: Option<Duration>,
}

impl Event {
    /// Creates an event without any fields.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the data of the event.
    ///
    /// Multi-line data is sent as one `data:` line per line, which the client
    /// reassembles into the original value.
    pub fn data(mut self, data: impl Into<String>) -> Self {
        self.data = Some(data.into());
        self
    }

    /// Serializes `value` as JSON and sets it as the data of the event.
    ///
    /// # Errors
    ///
    /// Returns an error if `value` cannot be serialized.
    pub fn json_data<T>(self, value: &T) -> Result<Self>
    where
        T: Serialize + ?Sized,
    {
        Ok(self.data(serde_json::to_string(value).map_err(Error::from)?))
    }

    /// Sets the event type, dispatched by an `EventSource` to the listener
    /// registered for it. Clients treat an event without a type as `message`.
    pub fn event(mut self, event: impl Into<String>) -> Self {
        self.kind = Some(event.into());
        self
    }

    /// Sets the event id, which the client echoes in the `Last-Event-ID`
    /// header when it reconnects. Read it with [`last_event_id`] to resume
    /// the stream.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the reconnection delay a client waits before it reconnects after
    /// losing the connection.
    pub fn retry(mut self, retry: Duration) -> Self {
        self.retry = Some(retry);
        self
    }

    /// Sets a comment, which a client ignores. Keep-alive events are
    /// comments, as are markers meant only for reading the raw stream.
    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Serializes the event into the `text/event-stream` wire format.
    ///
    /// # Errors
    ///
    /// Returns an [`InvalidEventError`] for a field whose value cannot be
    /// represented in the format.
    pub(super) fn serialize(&self) -> Result<Bytes> {
        let mut buffer = BytesMut::new();
        if let Some(comment) = &self.comment {
            for line in lines(comment) {
                put_field(&mut buffer, "", line);
            }
        }
        if let Some(kind) = &self.kind {
            if kind.contains(['\r', '\n']) {
                return Err(InvalidEventError::new("the event type contains a line break").into());
            }
            put_field(&mut buffer, "event", kind);
        }
        if let Some(data) = &self.data {
            for line in lines(data) {
                put_field(&mut buffer, "data", line);
            }
        }
        if let Some(id) = &self.id {
            if id.contains(['\r', '\n', '\0']) {
                return Err(InvalidEventError::new(
                    "the event id contains a line break or null character",
                )
                .into());
            }
            put_field(&mut buffer, "id", id);
        }
        if let Some(retry) = &self.retry {
            put_field(&mut buffer, "retry", &retry.as_millis().to_string());
        }
        buffer.extend_from_slice(b"\n");
        Ok(buffer.freeze())
    }
}

/// The error produced when an [`Event`] field cannot be represented in the
/// `text/event-stream` wire format.
#[derive(Debug)]
pub struct InvalidEventError {
    description: &'static str,
}

impl InvalidEventError {
    fn new(description: &'static str) -> Self {
        Self { description }
    }

    /// Returns the description of the invalid field.
    #[must_use]
    pub fn description(&self) -> &str {
        self.description
    }
}

impl fmt::Display for InvalidEventError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid server-sent event: {}", self.description)
    }
}

impl std::error::Error for InvalidEventError {}

/// Returns the `Last-Event-ID` header of the current request, or [`None`]
/// when it is absent or not valid UTF-8.
///
/// An `EventSource` sends this header when it reconnects to an event stream,
/// carrying the [`id`](Event::id) of the last event it received. Use it to
/// resume the stream instead of replaying it from the start.
///
/// # Examples
///
/// ```rust
/// use topcoat::{context::Cx, router::content::sse::last_event_id};
///
/// async fn resume_point(cx: &Cx) -> u64 {
///     last_event_id(cx)
///         .and_then(|id| id.parse().ok())
///         .unwrap_or(0)
/// }
/// ```
#[inline]
#[must_use]
pub fn last_event_id(cx: &Cx) -> Option<&str> {
    headers(cx).get("last-event-id")?.to_str().ok()
}

/// Appends a `field: value` line to the buffer; an empty field name makes the
/// line a comment.
fn put_field(buffer: &mut BytesMut, name: &str, value: &str) {
    buffer.extend_from_slice(name.as_bytes());
    buffer.extend_from_slice(b": ");
    buffer.extend_from_slice(value.as_bytes());
    buffer.extend_from_slice(b"\n");
}

/// Iterates over the lines of `value`, splitting on the `\r\n`, `\r`, and
/// `\n` line terminators of the `text/event-stream` format.
fn lines(value: &str) -> impl Iterator<Item = &str> {
    let mut rest = Some(value);
    std::iter::from_fn(move || {
        let current = rest.take()?;
        let Some(index) = current.find(['\r', '\n']) else {
            return Some(current);
        };
        let terminator = if current[index..].starts_with("\r\n") {
            2
        } else {
            1
        };
        rest = Some(&current[index + terminator..]);
        Some(&current[..index])
    })
}

#[cfg(test)]
mod tests {
    use http::Request;
    use topcoat_core::context::CxTestBuilder;

    use super::*;

    fn serialize(event: &Event) -> String {
        String::from_utf8(event.serialize().unwrap().to_vec()).unwrap()
    }

    // -- serialization --

    #[test]
    fn data_becomes_a_data_field() {
        assert_eq!(serialize(&Event::new().data("hi")), "data: hi\n\n");
    }

    #[test]
    fn an_empty_event_is_a_blank_line() {
        assert_eq!(serialize(&Event::new()), "\n");
    }

    #[test]
    fn every_field_is_serialized() {
        let event = Event::new()
            .comment("a comment")
            .event("tick")
            .data("hi")
            .id("1")
            .retry(Duration::from_secs(2));
        assert_eq!(
            serialize(&event),
            ": a comment\nevent: tick\ndata: hi\nid: 1\nretry: 2000\n\n"
        );
    }

    #[test]
    fn multi_line_data_becomes_one_field_per_line() {
        // The client joins the lines with `\n`, reassembling the original.
        assert_eq!(
            serialize(&Event::new().data("a\nb\r\nc\rd")),
            "data: a\ndata: b\ndata: c\ndata: d\n\n"
        );
        assert_eq!(
            serialize(&Event::new().data("trailing\n")),
            "data: trailing\ndata: \n\n"
        );
    }

    #[test]
    fn multi_line_comments_become_one_comment_per_line() {
        assert_eq!(serialize(&Event::new().comment("a\nb")), ": a\n: b\n\n");
    }

    #[test]
    fn json_data_serializes_the_value() {
        let event = Event::new()
            .json_data(&serde_json::json!({ "count": 3 }))
            .unwrap();
        assert_eq!(serialize(&event), "data: {\"count\":3}\n\n");
    }

    #[test]
    fn line_breaks_in_the_event_type_are_an_error() {
        let error = Event::new().event("a\nb").serialize().unwrap_err();
        assert!(error.downcast_ref::<InvalidEventError>().is_some());
    }

    #[test]
    fn line_breaks_and_null_in_the_id_are_an_error() {
        for id in ["a\nb", "a\rb", "a\0b"] {
            let error = Event::new().id(id).serialize().unwrap_err();
            assert!(error.downcast_ref::<InvalidEventError>().is_some());
        }
    }

    // -- last_event_id --

    #[test]
    fn last_event_id_reads_the_header() {
        let request = Request::builder()
            .uri("/events")
            .header("last-event-id", "42")
            .body(())
            .unwrap();
        let (parts, ()) = request.into_parts();
        let cx = CxTestBuilder::new().request_context(parts).build();
        assert_eq!(last_event_id(&cx), Some("42"));
    }

    #[test]
    fn a_missing_last_event_id_is_none() {
        let (parts, ()) = Request::builder()
            .uri("/events")
            .body(())
            .unwrap()
            .into_parts();
        let cx = CxTestBuilder::new().request_context(parts).build();
        assert_eq!(last_event_id(&cx), None);
    }
}
