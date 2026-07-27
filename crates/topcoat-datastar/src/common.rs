use std::time::Duration;

use futures_util::stream;
use topcoat_core::{context::Cx, error::Result};
use topcoat_router::{
    IntoResponse, Response,
    content::sse::{Event, Sse},
};

pub(crate) const PATCH_ELEMENTS_EVENT: &str = "datastar-patch-elements";
pub(crate) const PATCH_SIGNALS_EVENT: &str = "datastar-patch-signals";

/// Assembles a Datastar event of the given `kind`, joining the already
/// prefixed data `lines`.
pub(crate) fn event(
    kind: &str,
    id: Option<String>,
    retry: Option<Duration>,
    lines: &[String],
) -> Event {
    let mut event = Event::new().event(kind);

    if let Some(id) = id {
        event = event.id(id);
    }

    if let Some(retry) = retry {
        event = event.retry(retry);
    }

    if !lines.is_empty() {
        event = event.data(lines.join("\n"));
    }

    event
}

/// Builds an [`Sse`] response that sends `event` and ends the stream.
pub(crate) fn single_event_response(event: Event, cx: &Cx) -> Result<Response> {
    Sse::new(stream::iter([Ok::<_, topcoat_core::error::Error>(event)])).into_response(cx)
}
