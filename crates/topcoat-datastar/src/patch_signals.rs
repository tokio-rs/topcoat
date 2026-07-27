use std::time::Duration;

use serde::Serialize;
use topcoat_core::{context::Cx, error::Result};
use topcoat_router::{IntoResponse, Response, content::sse::Event};

use crate::common;

/// A `datastar-patch-signals` event that patches signals into the browser's
/// signal store.
///
/// The payload is a JSON object merged into the existing signals; a signal set
/// to `null` is removed. [`json`](Self::json) serializes the payload from any
/// [`Serialize`] value, and [`new`](Self::new) takes an already encoded
/// string.
///
/// The event converts [`Into<Event>`](Event) for sending over an
/// [`Sse`](topcoat_router::content::sse::Sse) stream. Returned from a handler
/// on its own, it responds as a stream that sends this one event and ends.
///
/// # Examples
///
/// ```rust
/// use serde::Serialize;
/// use topcoat::datastar::PatchSignals;
///
/// #[derive(Serialize)]
/// struct Progress {
///     percent: u8,
/// }
///
/// # fn example() -> topcoat::Result<PatchSignals> {
/// // `data: signals {"percent":80}`
/// let patch = PatchSignals::json(&Progress { percent: 80 })?;
/// # Ok(patch)
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct PatchSignals {
    signals: String,
    only_if_missing: bool,
    id: Option<String>,
    retry: Option<Duration>,
}

impl PatchSignals {
    /// Creates a patch from an already JSON-encoded `signals` object.
    pub fn new(signals: impl Into<String>) -> Self {
        Self {
            signals: signals.into(),
            only_if_missing: false,
            id: None,
            retry: None,
        }
    }

    /// Creates a patch by serializing `signals` to JSON.
    ///
    /// # Errors
    ///
    /// Returns an error when `signals` cannot be serialized.
    pub fn json<T>(signals: &T) -> Result<Self>
    where
        T: Serialize + ?Sized,
    {
        Ok(Self::new(serde_json::to_string(signals)?))
    }

    /// Only patches signals that do not exist yet, leaving present values
    /// untouched.
    pub fn only_if_missing(mut self, only_if_missing: bool) -> Self {
        self.only_if_missing = only_if_missing;
        self
    }

    /// Sets the event `id`, echoed back in `Last-Event-ID` when a lost stream
    /// reconnects.
    pub fn id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Sets the reconnection time the browser waits after losing the stream.
    pub fn retry(mut self, retry: Duration) -> Self {
        self.retry = Some(retry);
        self
    }
}

impl From<PatchSignals> for Event {
    fn from(patch: PatchSignals) -> Self {
        let mut lines = Vec::new();

        if patch.only_if_missing {
            lines.push("onlyIfMissing true".to_owned());
        }

        for line in patch.signals.lines() {
            lines.push(format!("signals {line}"));
        }

        common::event(common::PATCH_SIGNALS_EVENT, patch.id, patch.retry, &lines)
    }
}

impl IntoResponse for PatchSignals {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        common::single_event_response(self.into(), cx)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use topcoat_core::context::Cx;
    use topcoat_router::to_bytes;

    use super::*;

    async fn wire_format(patch: PatchSignals) -> String {
        let response = patch
            .into_response(&Cx::default())
            .expect("the response should build");

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("reading the response body");
        String::from_utf8(body.into()).expect("the body should be UTF-8")
    }

    #[tokio::test]
    async fn patch_sends_prefixed_signal_lines() {
        assert_eq!(
            wire_format(PatchSignals::new(r#"{"count":1}"#)).await,
            "event: datastar-patch-signals\n\
             data: signals {\"count\":1}\n\n"
        );
    }

    #[tokio::test]
    async fn json_serializes_the_signals() {
        let patch = PatchSignals::json(&json!({ "count": 2 })).expect("valid JSON");

        assert_eq!(
            wire_format(patch).await,
            "event: datastar-patch-signals\n\
             data: signals {\"count\":2}\n\n"
        );
    }

    #[tokio::test]
    async fn only_if_missing_precedes_the_signals() {
        let patch = PatchSignals::new(r#"{"theme":"dark"}"#).only_if_missing(true);

        assert_eq!(
            wire_format(patch).await,
            "event: datastar-patch-signals\n\
             data: onlyIfMissing true\n\
             data: signals {\"theme\":\"dark\"}\n\n"
        );
    }
}
