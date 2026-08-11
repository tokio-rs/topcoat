use std::fmt::Write;

use topcoat_core::error::{Error, Result};

use crate::{
    format::Formatter,
    html::HtmlContext,
    identity::Identity,
    pass::{children::ViewToken, scope::PassScope},
};

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

    /// Places a content view at this position: emits its marker so its
    /// output appears here.
    ///
    /// # Errors
    ///
    /// A content render that failed delivers its error at placement, so the
    /// placer propagates it with `?` or catches it, the way a layout catches
    /// its slot.
    ///
    /// # Panics
    ///
    /// Panics if the token is placed twice in one pass: one slot cannot fill
    /// two positions.
    pub fn place(&mut self, token: ViewToken) -> Result<(), Error> {
        let hash = token.identity.hash();
        let error = PassScope::with(|state| {
            let pass = state.pass;
            let content_state = state.content.get_mut(&hash)?;
            assert!(
                content_state.placed_pass < pass,
                "a view was placed twice in one pass"
            );
            content_state.placed_pass = pass;
            let error = content_state.error.take();
            if error.is_some() {
                state.stashed -= 1;
            }
            error
        });
        if let Some(error) = error {
            return Err(error);
        }
        self.child_marker(token.identity);
        Ok(())
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
