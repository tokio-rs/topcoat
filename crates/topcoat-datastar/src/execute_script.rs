use std::time::Duration;

use topcoat_core::{context::Cx, error::Result};
use topcoat_router::{IntoResponse, Response, content::sse::Event};

use crate::{ElementPatchMode, PatchElements, common};

/// An event that executes JavaScript in the browser.
///
/// Sugar for a [`PatchElements`] that appends a `<script>` element to the
/// `body`. By default the element removes itself after running; disable that
/// with [`auto_remove`](Self::auto_remove), and add attributes to the element
/// with [`attributes`](Self::attributes).
///
/// The event converts [`Into<Event>`](Event) for sending over an
/// [`Sse`](topcoat_router::content::sse::Sse) stream. Returned from a handler
/// on its own, it responds as a stream that sends this one event and ends.
///
/// # Examples
///
/// ```rust
/// use topcoat::datastar::ExecuteScript;
///
/// let script = ExecuteScript::new("console.log('saved')");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use]
pub struct ExecuteScript {
    script: String,
    auto_remove: bool,
    attributes: Vec<String>,
    id: Option<String>,
    retry: Option<Duration>,
}

impl ExecuteScript {
    /// Creates an event that executes the given JavaScript `script`.
    pub fn new(script: impl Into<String>) -> Self {
        Self {
            script: script.into(),
            auto_remove: true,
            attributes: Vec::new(),
            id: None,
            retry: None,
        }
    }

    /// Sets whether the script element removes itself after running.
    pub fn auto_remove(mut self, auto_remove: bool) -> Self {
        self.auto_remove = auto_remove;
        self
    }

    /// Adds attributes to the script element. Each entry is a complete
    /// attribute, such as `type="module"`.
    pub fn attributes(mut self, attributes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.attributes = attributes.into_iter().map(Into::into).collect();
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

impl From<ExecuteScript> for Event {
    fn from(script: ExecuteScript) -> Self {
        let mut element = String::from("<script");

        if script.auto_remove {
            element.push_str(r#" data-effect="el.remove()""#);
        }

        for attribute in &script.attributes {
            element.push(' ');
            element.push_str(attribute);
        }

        element.push('>');
        element.push_str(&script.script);
        element.push_str("</script>");

        let mut patch = PatchElements::new(element)
            .selector("body")
            .mode(ElementPatchMode::Append);

        if let Some(id) = script.id {
            patch = patch.id(id);
        }

        if let Some(retry) = script.retry {
            patch = patch.retry(retry);
        }

        patch.into()
    }
}

impl IntoResponse for ExecuteScript {
    fn into_response(self, cx: &Cx) -> Result<Response> {
        common::single_event_response(self.into(), cx)
    }
}

#[cfg(test)]
mod tests {
    use topcoat_core::context::Cx;
    use topcoat_router::to_bytes;

    use super::*;

    async fn wire_format(script: ExecuteScript) -> String {
        let response = script
            .into_response(&Cx::default())
            .expect("the response should build");

        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("reading the response body");
        String::from_utf8(body.into()).expect("the body should be UTF-8")
    }

    #[tokio::test]
    async fn script_appends_a_self_removing_element_to_the_body() {
        assert_eq!(
            wire_format(ExecuteScript::new("console.log('hi')")).await,
            "event: datastar-patch-elements\n\
             data: selector body\n\
             data: mode append\n\
             data: elements <script data-effect=\"el.remove()\">console.log('hi')</script>\n\n"
        );
    }

    #[tokio::test]
    async fn attributes_and_auto_remove_shape_the_element() {
        let script = ExecuteScript::new("import('/app.js')")
            .auto_remove(false)
            .attributes([r#"type="module""#]);

        assert_eq!(
            wire_format(script).await,
            "event: datastar-patch-elements\n\
             data: selector body\n\
             data: mode append\n\
             data: elements <script type=\"module\">import('/app.js')</script>\n\n"
        );
    }

    #[tokio::test]
    async fn multiline_scripts_span_multiple_element_lines() {
        assert_eq!(
            wire_format(ExecuteScript::new("let a = 1\nconsole.log(a)").auto_remove(false)).await,
            "event: datastar-patch-elements\n\
             data: selector body\n\
             data: mode append\n\
             data: elements <script>let a = 1\n\
             data: elements console.log(a)</script>\n\n"
        );
    }
}
