pub use futures_util::{future::try_join_all, try_join};
use topcoat_core::context::Cx;

use crate::{
    Attribute, AttributeKeyViewParts, AttributeValueViewParts, AttributeViewParts,
    ElementNameViewParts, HtmlContext, NodeViewParts, PartsWriter, Unescaped, View,
    render::{Memory, with_memory},
};

/// Builds a view's instruction block in one synchronous burst.
///
/// Records the entry address in the active scope's memory, runs `f` to push
/// the block's instructions, terminates the block with a return instruction,
/// and returns the view handle. `f` must not build other views; nested views
/// are built first and spliced into the block with [`__view`].
///
/// # Panics
///
/// Panics if no view scope is active on the current task.
pub fn __build_view(f: impl FnOnce(&mut Memory)) -> View {
    with_memory(|memory| {
        let entry = memory.next_ptr();
        f(memory);
        memory.push_ret();
        View::from_scope(memory.id(), entry)
    })
}

#[inline]
pub fn __unescaped(_cx: &Cx, memory: &mut Memory, s: &'static str) {
    memory.push_static_str(s, HtmlContext::Unescaped);
}

#[inline]
pub fn __view(_cx: &Cx, memory: &mut Memory, view: View) {
    memory.push_view(view);
}

#[inline]
pub fn __node(cx: &Cx, memory: &mut Memory, node: impl NodeViewParts) {
    node.into_view_parts(cx, &mut PartsWriter::new(memory, HtmlContext::Text));
}

#[inline]
pub fn __element_name(cx: &Cx, memory: &mut Memory, element_name: impl ElementNameViewParts) {
    element_name.into_view_parts(cx, &mut PartsWriter::new(memory, HtmlContext::ElementName));
}

#[inline]
pub fn __attribute_key(cx: &Cx, memory: &mut Memory, attribute_key: impl AttributeKeyViewParts) {
    attribute_key.into_view_parts(cx, &mut PartsWriter::new(memory, HtmlContext::AttributeKey));
}

#[inline]
pub fn __attribute_value(
    cx: &Cx,
    memory: &mut Memory,
    attribute_value: impl AttributeValueViewParts,
) {
    attribute_value.into_view_parts(
        cx,
        &mut PartsWriter::new(memory, HtmlContext::AttributeValue),
    );
}

#[inline]
pub fn __attribute(
    cx: &Cx,
    memory: &mut Memory,
    (key, value): (impl AttributeKeyViewParts, impl AttributeValueViewParts),
) {
    __attributes(cx, memory, Attribute::new(key, value));
}

#[inline]
pub fn __attribute_unescaped(
    cx: &Cx,
    memory: &mut Memory,
    (key, value): (&'static str, impl AttributeValueViewParts),
) {
    __attributes(
        cx,
        memory,
        Attribute::new(Unescaped::new_unchecked(key), value),
    );
}

#[inline]
pub fn __attributes(cx: &Cx, memory: &mut Memory, attributes: impl AttributeViewParts) {
    // Whole-attribute values do their own context transitions between
    // keys and values; the attribute-value context here is the safe
    // default for any text pushed directly.
    attributes.into_view_parts(
        cx,
        &mut PartsWriter::new(memory, HtmlContext::AttributeValue),
    );
}
