use std::fmt::Write;

use crate::{format::Formatter, html::HtmlContext, identity::Identity, pass::scope::PassScope};

/// The output buffer of one render.
///
/// A component's render writes one of these per pass and hands it to
/// [`Mount::finish_render`]. Child components do not write here; their
/// position is held by a marker and their output lives in their own slot.
#[derive(Default)]
pub struct RenderBuffer {
    html: String,
}

impl RenderBuffer {
    #[must_use]
    pub fn new() -> Self {
        RenderBuffer::default()
    }

    /// Appends text, escaped for HTML text position.
    pub fn text(&mut self, value: &str) {
        let mut f = Formatter::new(&mut self.html);
        HtmlContext::Text.writer(&mut f).write_str(value);
    }

    /// Appends trusted markup verbatim.
    pub fn markup(&mut self, value: &str) {
        self.html.push_str(value);
    }

    /// Appends the output marker of a child component.
    pub(crate) fn child_marker(&mut self, identity: Identity) {
        write!(self.html, "<!--tc:{:032x}-->", identity.hash())
            .expect("writing to a string cannot fail");
    }

    pub(crate) fn into_html(self) -> String {
        self.html
    }
}

/// The registration of one live component instance.
///
/// Created at the top of a component's body; holds the component's slot in
/// the request for as long as the component future lives. Dropping it, which
/// happens when the future completes or is evicted, removes the component.
pub struct Mount {
    identity: Identity,
}

impl Mount {
    /// Stores the output of this pass's render and marks the component as
    /// rendered for the current pass.
    ///
    /// # Panics
    ///
    /// Panics if no pass is running on the current task.
    pub fn finish_render(&self, out: RenderBuffer) {
        let hash = self.identity.hash();
        PassScope::with(|state| {
            let pass = state.pass;
            let slot = state.slots.entry(hash).or_default();
            slot.html = out.into_html();
            slot.rendered_pass = pass;
            state.births.remove(&hash);
        });
    }

    pub(crate) fn identity(&self) -> Identity {
        self.identity
    }
}

impl Drop for Mount {
    fn drop(&mut self) {
        let hash = self.identity.hash();
        let _ = PassScope::try_with(|state| {
            state.slots.remove(&hash);
            state.births.remove(&hash);
        });
    }
}

/// Registers the component running at the current identity and returns its
/// mount.
///
/// # Panics
///
/// Panics if the current identity is ambiguous (a repeated invocation is
/// missing its `key`) or if no pass is running on the current task.
#[must_use]
#[track_caller]
pub fn mount() -> Mount {
    let identity = Identity::current();
    PassScope::with(|state| {
        state.slots.entry(identity.hash()).or_default();
    });
    Mount { identity }
}
