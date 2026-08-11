use topcoat_core::{
    context::Cx,
    error::{Error, Result},
};

use crate::{
    Attribute, AttributeKeyViewParts, AttributeValueViewParts, AttributeViewParts,
    ElementNameViewParts, NodeViewParts, Unescaped, View,
    buffer::{InstructionPtr, Renderer, ViewBuffer},
    format::Formatter,
    html::HtmlContext,
    identity::Identity,
    part::PartsWriter,
    pass::{children::ViewToken, scope::PassScope},
};

/// The output buffer of one render.
///
/// Backed by an instruction buffer, so every value type the view macros
/// support in a position works on the pass path unchanged. A render is one
/// synchronous burst, which is exactly the buffer's contiguity contract;
/// [`Mount::finish_render`] executes the block into the component's output
/// slot. Child components do not write here; their position is held by a
/// marker and their output lives in their own slot.
pub struct RenderBuffer {
    buffer: ViewBuffer,
    entry: InstructionPtr,
}

impl RenderBuffer {
    #[must_use]
    pub fn new() -> Self {
        let buffer = ViewBuffer::new();
        let entry = buffer.next_ptr();
        RenderBuffer { buffer, entry }
    }

    fn parts(&mut self, context: HtmlContext) -> PartsWriter<'_> {
        PartsWriter::new(&mut self.buffer, context)
    }

    fn cx() -> Cx {
        PassScope::with(|state| state.cx.detach())
    }

    /// Appends text, escaped for HTML text position.
    pub fn text(&mut self, value: &str) {
        self.parts(HtmlContext::Text).push_str(value);
    }

    /// Appends trusted markup verbatim.
    pub fn markup(&mut self, value: &str) {
        self.parts(HtmlContext::Unescaped).push_str(value);
    }

    /// Appends a literal markup segment, verbatim.
    ///
    /// The segment is passed as `&"..."` so it stays out of the buffer's
    /// constants.
    pub fn markup_static(&mut self, value: &'static &'static str) {
        self.parts(HtmlContext::Unescaped).push_promoted_str(value);
    }

    /// Appends a value in a text node position.
    pub fn node(&mut self, value: impl NodeViewParts) {
        let cx = Self::cx();
        let mut parts = self.parts(HtmlContext::Text);
        value.into_view_parts(&cx, &mut parts);
    }

    /// Appends a value in an element name position.
    pub fn element_name(&mut self, value: impl ElementNameViewParts) {
        let cx = Self::cx();
        let mut parts = self.parts(HtmlContext::ElementName);
        value.into_view_parts(&cx, &mut parts);
    }

    /// Appends a value in an attribute key position.
    pub fn attribute_key(&mut self, value: impl AttributeKeyViewParts) {
        let cx = Self::cx();
        let mut parts = self.parts(HtmlContext::AttributeKey);
        value.into_view_parts(&cx, &mut parts);
    }

    /// Appends a value in an attribute value position.
    pub fn attribute_value(&mut self, value: impl AttributeValueViewParts) {
        let cx = Self::cx();
        let mut parts = self.parts(HtmlContext::AttributeValue);
        value.into_view_parts(&cx, &mut parts);
    }

    /// Appends a whole attribute from a key and value pair.
    pub fn attribute(
        &mut self,
        (key, value): (impl AttributeKeyViewParts, impl AttributeValueViewParts),
    ) {
        self.attributes(Attribute::new(key, value));
    }

    /// Appends a whole attribute from a trusted literal key and a value.
    pub fn attribute_unescaped(
        &mut self,
        (key, value): (&'static str, impl AttributeValueViewParts),
    ) {
        self.attributes(Attribute::new(Unescaped::new_unchecked(key), value));
    }

    /// Appends a value covering whole attributes, keys and values.
    pub fn attributes(&mut self, attributes: impl AttributeViewParts) {
        let cx = Self::cx();
        // Whole-attribute values do their own context transitions between
        // keys and values; the attribute-value context is the safe default
        // for any text pushed directly.
        let mut parts = self.parts(HtmlContext::AttributeValue);
        attributes.into_view_parts(&cx, &mut parts);
    }

    /// Splices an already built view value.
    ///
    /// # Panics
    ///
    /// Panics if the view is a handle into a still building `view!`
    /// invocation instead of a sealed value.
    pub fn view(&mut self, view: View) {
        self.parts(HtmlContext::Text).push_view(view);
    }

    /// Appends the output marker of a child component.
    pub(crate) fn child_marker(&mut self, identity: Identity) {
        self.parts(HtmlContext::Unescaped)
            .push_string(format!("<!--tc:{:032x}-->", identity.hash()));
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

    /// Terminates the block and executes it into its output.
    fn finish(mut self, cx: &Cx) -> FinishedRender {
        self.buffer.push_ret();
        let mut html = String::new();
        let mut f = Formatter::new(&mut html);
        Renderer::new(&self.buffer, self.entry).execute(cx, &mut f);
        #[cfg(feature = "http")]
        let (status_code, headers) = f.into_recorded();
        #[cfg(not(feature = "http"))]
        drop(f);
        FinishedRender {
            html,
            #[cfg(feature = "http")]
            status_code,
            #[cfg(feature = "http")]
            headers,
        }
    }
}

impl Default for RenderBuffer {
    fn default() -> Self {
        RenderBuffer::new()
    }
}

/// One executed render: the output and the response metadata it declared.
struct FinishedRender {
    html: String,
    #[cfg(feature = "http")]
    status_code: Option<http::StatusCode>,
    #[cfg(feature = "http")]
    headers: http::HeaderMap,
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
    /// Executes the render and stores its output, marking the component as
    /// rendered for the current pass. Response metadata the render declared
    /// is recorded, first declaration wins.
    ///
    /// # Panics
    ///
    /// Panics if no pass is running on the current task.
    pub fn finish_render(&self, out: RenderBuffer) {
        let cx = PassScope::with(|state| state.cx.detach());
        let finished = out.finish(&cx);
        let hash = self.identity.hash();
        PassScope::with(|state| {
            let pass = state.pass;
            let slot = state.slots.entry(hash).or_default();
            slot.html = finished.html;
            slot.rendered_pass = pass;
            state.births.remove(&hash);
            #[cfg(feature = "http")]
            {
                if let Some(status_code) = finished.status_code {
                    state.status_code.get_or_insert(status_code);
                }
                if state.headers.is_empty() {
                    state.headers = finished.headers;
                } else {
                    for name in finished.headers.keys() {
                        if !state.headers.contains_key(name) {
                            for value in finished.headers.get_all(name) {
                                state.headers.append(name.clone(), value.clone());
                            }
                        }
                    }
                }
            }
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
