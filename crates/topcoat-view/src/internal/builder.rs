use topcoat_core::context::Cx;

use crate::{
    Attribute, AttributeKeyViewParts, AttributeValueViewParts, AttributeViewParts, HtmlContext,
    NodeViewParts, PartsWriter, Unescaped, ViewHandle, buffer::OpenBlock,
    html::ElementNameViewParts,
};

/// The handle a template fills its block through: the request context plus
/// the block under construction in the buffer of the build.
///
/// A builder is opened at the start of a template and closed at its end,
/// and the template's code runs in between, pushing the block's parts in
/// source order. Each method pushes one part, sealing it with the
/// [`HtmlContext`] of the position it fills by dispatching the matching
/// `*ViewParts` trait. Nothing else can build into the buffer while the
/// builder holds it, so code that waits, like an `await`, runs between
/// [`suspend`](Self::suspend) and [`resume`](Self::resume).
pub struct Builder<'a> {
    cx: &'a Cx,
    block: OpenBlock,
}

impl<'a> Builder<'a> {
    /// Starts a block in the buffer of the build.
    ///
    /// # Panics
    ///
    /// Panics if no view is building on the current task.
    #[must_use]
    pub fn open(cx: &'a Cx) -> Self {
        Self {
            cx,
            block: OpenBlock::open(),
        }
    }

    /// Terminates the block and returns the handle to it.
    ///
    /// # Panics
    ///
    /// Panics if the block is suspended.
    #[must_use]
    pub fn close(self) -> ViewHandle {
        self.block.close()
    }

    /// Hands the buffer back to the build until [`resume`](Self::resume),
    /// so the template may wait without holding it.
    ///
    /// # Panics
    ///
    /// Panics if the block is suspended already.
    #[inline]
    pub fn suspend(&mut self) {
        self.block.suspend();
    }

    /// Takes the buffer back after [`suspend`](Self::suspend) and continues
    /// the block.
    ///
    /// # Panics
    ///
    /// Panics if the block is not suspended, or if no view is building on
    /// the current task.
    #[inline]
    pub fn resume(&mut self) {
        self.block.resume();
    }

    /// Suspends the block and returns the builder, to chain
    /// [`resumed`](Self::resumed) around an expression that awaits.
    ///
    /// # Panics
    ///
    /// Panics if the block is suspended already.
    #[inline]
    pub fn suspended(&mut self) -> &mut Self {
        self.suspend();
        self
    }

    /// Resumes the block and passes `value` through.
    ///
    /// Written as `__b.suspended().resumed(expr)`, the block is suspended
    /// while `expr` evaluates and the temporaries of `expr` live as long as
    /// the statement around the call, as they would without the wrapping.
    ///
    /// # Panics
    ///
    /// Panics if the block is not suspended, or if no view is building on
    /// the current task.
    #[inline]
    pub fn resumed<T>(&mut self, value: T) -> T {
        self.resume();
        value
    }

    /// Appends one block to the buffer of the build, pushing its parts
    /// through the builder handed to `f`, and returns the handle to it.
    ///
    /// # Panics
    ///
    /// Panics if no view is building on the current task.
    pub fn block(cx: &Cx, f: impl FnOnce(&mut Builder<'_>)) -> ViewHandle {
        let mut builder = Builder::open(cx);
        f(&mut builder);
        builder.close()
    }

    /// Builds a self-contained view from the parts `f` pushes, in a build
    /// of its own.
    #[cfg(test)]
    pub(crate) fn build(cx: &Cx, f: impl FnOnce(&mut Builder<'_>)) -> ViewHandle {
        use crate::buffer::{ViewBuffer, ViewBufferScope};

        let mut slot = Some(Box::new(ViewBuffer::new()));
        let view = {
            let _scope = ViewBufferScope::install(&mut slot);
            Builder::block(cx, f)
        };
        view.seal(*slot.expect("the buffer was swapped back on exit"))
    }

    /// Returns a writer over the block, in text context.
    ///
    /// For compositions that push through the writer directly instead of a
    /// position method, like the runtime's JavaScript views.
    #[inline]
    pub fn parts(&mut self) -> PartsWriter<'_> {
        PartsWriter::new(self.block.buffer(), HtmlContext::Text)
    }

    /// Returns a writer over the block sealing for `context`.
    #[inline]
    fn parts_in(&mut self, context: HtmlContext) -> PartsWriter<'_> {
        PartsWriter::new(self.block.buffer(), context)
    }

    /// Appends a literal markup segment, verbatim.
    ///
    /// The segment is passed as `&"..."` so it stays out of the buffer's
    /// constants.
    #[inline]
    pub fn markup(&mut self, s: &'static &'static str) {
        self.parts().push_promoted_str_unescaped(s);
    }

    /// Appends a value in a text node position.
    #[inline]
    pub fn node(&mut self, value: impl NodeViewParts) {
        let cx = self.cx;
        value.into_view_parts(cx, &mut self.parts_in(HtmlContext::Text));
    }

    /// Appends a value in an element name position.
    #[inline]
    pub fn element_name(&mut self, value: impl ElementNameViewParts) {
        let cx = self.cx;
        value.into_view_parts(cx, &mut self.parts_in(HtmlContext::ElementName));
    }

    /// Appends a value in an attribute key position.
    #[inline]
    pub fn attribute_key(&mut self, value: impl AttributeKeyViewParts) {
        let cx = self.cx;
        value.into_view_parts(cx, &mut self.parts_in(HtmlContext::AttributeKey));
    }

    /// Appends a value in an attribute value position.
    #[inline]
    pub fn attribute_value(&mut self, value: impl AttributeValueViewParts) {
        let cx = self.cx;
        value.into_view_parts(cx, &mut self.parts_in(HtmlContext::AttributeValue));
    }

    /// Appends a whole attribute from a key and value pair.
    #[inline]
    pub fn attribute(
        &mut self,
        (key, value): (impl AttributeKeyViewParts, impl AttributeValueViewParts),
    ) {
        self.attributes(Attribute::new(key, value));
    }

    /// Appends a whole attribute from a trusted literal key and a value.
    #[inline]
    pub fn attribute_unescaped(
        &mut self,
        (key, value): (&'static str, impl AttributeValueViewParts),
    ) {
        self.attributes(Attribute::new(Unescaped::new_unchecked(key), value));
    }

    /// Appends a value covering whole attributes, keys and values.
    #[inline]
    pub fn attributes(&mut self, attributes: impl AttributeViewParts) {
        let cx = self.cx;
        // Whole-attribute values do their own context transitions between
        // keys and values; the attribute-value context here is the safe
        // default for any text pushed directly.
        attributes.into_view_parts(cx, &mut self.parts_in(HtmlContext::AttributeValue));
    }
}
