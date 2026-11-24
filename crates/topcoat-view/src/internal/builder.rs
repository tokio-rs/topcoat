use topcoat_core::context::Cx;

use crate::{
    Attribute, AttributeKeyViewParts, AttributeValueViewParts, AttributeViewParts, HtmlContext,
    NodeViewParts, PartsWriter, PromotedStr, ViewHandle, buffer::ViewBufferScope,
    html::ElementNameViewParts,
};

/// The handle a template's burst pushes its parts through.
///
/// Wraps a [`PartsWriter`] with the request context and enters the HTML
/// context matching each position, so every value is escaped for where it
/// lands.
pub struct Builder<'a, 'b, 'c> {
    cx: &'a Cx,
    parts: &'b mut PartsWriter<'c>,
}

impl<'a, 'b, 'c> Builder<'a, 'b, 'c> {
    fn new(cx: &'a Cx, parts: &'b mut PartsWriter<'c>) -> Self {
        Self { cx, parts }
    }

    /// Builds a self-contained view in one synchronous burst, pushing its
    /// parts through the builder handed to `f`.
    #[cfg(test)]
    pub(crate) fn build(cx: &Cx, f: impl FnOnce(&mut Builder<'_, '_, '_>)) -> ViewHandle {
        crate::buffer::ViewBuffer::build(|parts| f(&mut Builder::new(cx, parts)))
    }

    /// Appends one view's instruction block to the buffer of the build in
    /// one synchronous burst, pushing its parts through the builder handed
    /// to `f`, and returns the handle to the block.
    ///
    /// `f` must not build other views; nested views are built first and
    /// spliced into the block with [`view`](Self::view).
    ///
    /// # Panics
    ///
    /// Panics if no view is building on the current task.
    pub fn block(cx: &Cx, f: impl FnOnce(&mut Builder<'_, '_, '_>)) -> ViewHandle {
        ViewBufferScope::with(|buffer| buffer.block(|parts| f(&mut Builder::new(cx, parts))))
    }
    /// Appends a literal markup segment, verbatim.
    ///
    /// The segment is passed as `&"..."` so it stays out of the buffer's
    /// constants.
    #[inline]
    pub fn markup(&mut self, s: &'static &'static str) {
        self.parts.push_promoted_str_unescaped(s);
    }

    /// Appends a value in a text node position.
    #[inline]
    pub fn node(&mut self, value: impl NodeViewParts) {
        let cx = self.cx;
        self.parts.in_context(HtmlContext::Text, |parts| {
            value.into_view_parts(cx, parts);
        });
    }

    /// Appends a value in an element name position.
    #[inline]
    pub fn element_name(&mut self, value: impl ElementNameViewParts) {
        let cx = self.cx;
        self.parts.in_context(HtmlContext::ElementName, |parts| {
            value.into_view_parts(cx, parts);
        });
    }

    /// Appends a value in an attribute key position.
    #[inline]
    pub fn attribute_key(&mut self, value: impl AttributeKeyViewParts) {
        let cx = self.cx;
        self.parts.in_context(HtmlContext::AttributeKey, |parts| {
            value.into_view_parts(cx, parts);
        });
    }

    /// Appends a value in an attribute value position.
    #[inline]
    pub fn attribute_value(&mut self, value: impl AttributeValueViewParts) {
        let cx = self.cx;
        self.parts.in_context(HtmlContext::AttributeValue, |parts| {
            value.into_view_parts(cx, parts);
        });
    }

    /// Appends a whole attribute from a key and value pair.
    #[inline]
    pub fn attribute(
        &mut self,
        (key, value): (impl AttributeKeyViewParts, impl AttributeValueViewParts),
    ) {
        self.attributes(Attribute::new(key, value));
    }

    /// Appends a whole attribute from its trusted literal opening and a
    /// value.
    ///
    /// The opening is the attribute's leading space, its name and `="`.
    /// Nothing is appended if the value marks the attribute as absent.
    #[inline]
    pub fn attribute_unescaped(
        &mut self,
        (opening, value): (PromotedStr, impl AttributeValueViewParts),
    ) {
        if value.attribute_present() {
            let cx = self.cx;
            self.parts.push_promoted_str_unescaped(opening.0);
            self.parts.in_context(HtmlContext::AttributeValue, |parts| {
                value.into_view_parts(cx, parts);
            });
            self.parts.push_promoted_str_unescaped(&"\"");
        }
    }

    /// Appends a value covering whole attributes, keys and values.
    #[inline]
    pub fn attributes(&mut self, attributes: impl AttributeViewParts) {
        let cx = self.cx;
        // Whole-attribute values do their own context transitions between
        // keys and values; the attribute-value context here is the safe
        // default for any text pushed directly.
        self.parts.in_context(HtmlContext::AttributeValue, |parts| {
            attributes.into_view_parts(cx, parts);
        });
    }

    /// Splices an already-built view.
    ///
    /// # Panics
    ///
    /// Panics if the view was built in a different, still building buffer.
    #[inline]
    pub fn view(&mut self, view: ViewHandle) {
        self.parts.push_view_handle(view);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(f: impl FnOnce(&mut Builder<'_, '_, '_>)) -> String {
        let cx = Cx::new();
        Builder::build(&cx, f).render(&cx)
    }

    #[test]
    fn attribute_unescaped_escapes_only_the_value() {
        let rendered = render(|b| b.attribute_unescaped((PromotedStr(&" data-x=\""), "a\"b")));
        assert_eq!(rendered, " data-x=\"a&quot;b\"");
    }

    #[test]
    fn attribute_unescaped_omits_an_absent_value() {
        let rendered = render(|b| b.attribute_unescaped((PromotedStr(&" disabled=\""), false)));
        assert_eq!(rendered, "");
    }
}
