use std::time::Duration;

use topcoat_core::{context::Cx, error::Result};
use topcoat_router::{IntoResponse, Response, content::sse::Event};

use crate::{ElementPatchMode, common};

/// A `datastar-patch-elements` event that patches HTML elements into the DOM.
///
/// By default the elements are morphed into the existing DOM, matched by their
/// `id` attribute. A [`selector`](Self::selector) targets other elements, and
/// a [`mode`](Self::mode) changes how they are applied.
///
/// The event converts [`Into<Event>`](Event) for sending over an
/// [`Sse`](topcoat_router::content::sse::Sse) stream. Returned from a handler
/// on its own, it responds as a stream that sends this one event and ends.
///
/// # Examples
///
/// ```rust
/// use topcoat::datastar::{ElementPatchMode, PatchElements};
///
/// let patch = PatchElements::new("<li>New entry</li>")
///     .selector("#feed")
///     .mode(ElementPatchMode::Prepend);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct PatchElements {
    elements: Option<String>,
    selector: Option<String>,
    mode: ElementPatchMode,
    use_view_transition: bool,
    id: Option<String>,
    retry: Option<Duration>,
}

impl PatchElements {
    /// Creates a patch that applies the given HTML `elements`.
    pub fn new(elements: impl Into<String>) -> Self {
        Self {
            elements: Some(elements.into()),
            selector: None,
            mode: ElementPatchMode::default(),
            use_view_transition: false,
            id: None,
            retry: None,
        }
    }

    /// Creates a patch that removes the elements matching `selector` from the
    /// DOM.
    pub fn remove(selector: impl Into<String>) -> Self {
        Self {
            elements: None,
            selector: Some(selector.into()),
            mode: ElementPatchMode::Remove,
            use_view_transition: false,
            id: None,
            retry: None,
        }
    }

    /// Targets the elements matching this CSS selector instead of matching by
    /// `id`.
    pub fn selector(mut self, selector: impl Into<String>) -> Self {
        self.selector = Some(selector.into());
        self
    }

    /// Sets how the elements are patched into the DOM.
    pub fn mode(mut self, mode: ElementPatchMode) -> Self {
        self.mode = mode;
        self
    }

    /// Patches the elements using the View Transition API.
    pub fn use_view_transition(mut self, use_view_transition: bool) -> Self {
        self.use_view_transition = use_view_transition;
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

impl From<PatchElements> for Event {
    fn from(patch: PatchElements) -> Self {
        let mut lines = Vec::new();

        if let Some(selector) = patch.selector {
            lines.push(format!("selector {selector}"));
        }

        if patch.mode != ElementPatchMode::default() {
            lines.push(format!("mode {}", patch.mode.as_str()));
        }

        if patch.use_view_transition {
            lines.push("useViewTransition true".to_owned());
        }

        if let Some(elements) = patch.elements {
            for line in elements.lines() {
                lines.push(format!("elements {line}"));
            }
        }

        common::event(common::PATCH_ELEMENTS_EVENT, patch.id, patch.retry, &lines)
    }
}

impl IntoResponse for PatchElements {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        common::single_event_response(self.into(), cx)
    }
}

#[cfg(test)]
mod tests {
    use topcoat_core::context::Cx;
    use topcoat_router::to_bytes;

    use super::*;

    async fn wire_format(patch: PatchElements) -> String {
        let response = patch
            .into_response(&Cx::default())
            .expect("the response should build");

        assert_eq!(
            response
                .headers()
                .get(http::header::CONTENT_TYPE)
                .map(http::HeaderValue::as_bytes),
            Some(b"text/event-stream".as_slice())
        );

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("reading the response body");
        String::from_utf8(body.into()).expect("the body should be UTF-8")
    }

    #[tokio::test]
    async fn patch_sends_prefixed_element_lines() {
        let body = wire_format(PatchElements::new(
            "<div id=\"a\">1</div>\n<div id=\"b\">2</div>",
        ))
        .await;

        assert_eq!(
            body,
            "event: datastar-patch-elements\n\
             data: elements <div id=\"a\">1</div>\n\
             data: elements <div id=\"b\">2</div>\n\n"
        );
    }

    #[tokio::test]
    async fn selector_mode_and_view_transition_precede_the_elements() {
        let patch = PatchElements::new("<li>new</li>")
            .selector("#feed")
            .mode(ElementPatchMode::Prepend)
            .use_view_transition(true);

        assert_eq!(
            wire_format(patch).await,
            "event: datastar-patch-elements\n\
             data: selector #feed\n\
             data: mode prepend\n\
             data: useViewTransition true\n\
             data: elements <li>new</li>\n\n"
        );
    }

    #[tokio::test]
    async fn remove_patches_without_elements() {
        assert_eq!(
            wire_format(PatchElements::remove("#stale")).await,
            "event: datastar-patch-elements\n\
             data: selector #stale\n\
             data: mode remove\n\n"
        );
    }

    #[tokio::test]
    async fn id_and_retry_drive_reconnection() {
        let patch = PatchElements::new("<div>x</div>")
            .id("42")
            .retry(Duration::from_secs(2));

        assert_eq!(
            wire_format(patch).await,
            "event: datastar-patch-elements\n\
             data: elements <div>x</div>\n\
             id: 42\n\
             retry: 2000\n\n"
        );
    }
}
