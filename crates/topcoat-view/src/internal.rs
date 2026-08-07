pub use futures_util::{
    future::{Either as __Either, try_join_all as __try_join_all},
    try_join as __try_join,
};
use topcoat_core::{context::Cx, error::Result};

use crate::{
    Attribute, AttributeKeyViewParts, AttributeValueViewParts, AttributeViewParts,
    ElementNameViewParts, HtmlContext, NodeViewParts, PartsWriter, Unescaped, View,
    render::{Memory, ViewSlot, with_memory},
};

/// Builds a view's instruction block in one synchronous burst.
///
/// Records the entry address in the active scope's memory, runs `f` to push
/// the block's instructions through a text-context writer, and terminates the
/// block with a return instruction. The returned view handle carries the
/// writer's accumulated size hint. `f` must not build other views; nested
/// views are built first and spliced into the block with [`__view`].
///
/// # Panics
///
/// Panics if no view scope is active on the current task.
pub fn __build_view(f: impl FnOnce(&mut PartsWriter<'_>)) -> View {
    with_memory(|memory| {
        let entry = memory.next_ptr();
        let mut parts = PartsWriter::new(memory, HtmlContext::Text);
        f(&mut parts);
        let size_hint = parts.size_hint();
        memory.push_ret();
        View::from_scope(memory.id(), entry, size_hint)
    })
}

/// Reserves a slot in the active scope's memory for a view that resolves
/// later.
///
/// A component whose children render components of their own passes the
/// placeholder view to its props so it can render concurrently with the
/// children; [`__fill_view`] redirects the slot once they resolve.
///
/// # Panics
///
/// Panics if no view scope is active on the current task.
#[must_use]
pub fn __reserve_view() -> (View, ViewSlot) {
    with_memory(Memory::reserve_view)
}

/// Redirects a reserved slot to `view`, resolving its placeholder.
///
/// # Panics
///
/// Panics if no view scope is active on the current task, if the slot or the
/// view belongs to a different scope, or if the slot was already filled.
pub fn __fill_view(slot: ViewSlot, view: View) {
    with_memory(|memory| memory.fill_view(slot, view));
}

/// Wraps an already-built view in a ready future.
///
/// Splices a view without components into a joined position, like the branch
/// of an `if` whose other branch renders components.
pub fn __ready_view(view: View) -> futures_util::future::Ready<Result<View>> {
    futures_util::future::ready(Ok(view))
}

/// Runs `f` with the writer sealing for a different context, then restores
/// the current context.
///
/// Out-of-crate macro expansions use this for compositions that span more
/// than one position, like the runtime's JavaScript views.
#[inline]
pub fn __in_context<R>(
    parts: &mut PartsWriter<'_>,
    context: HtmlContext,
    f: impl FnOnce(&mut PartsWriter<'_>) -> R,
) -> R {
    parts.in_context(context, f)
}

#[inline]
pub fn __unescaped(_cx: &Cx, parts: &mut PartsWriter<'_>, s: &'static str) {
    parts.push_static_str_unescaped(s);
}

#[inline]
pub fn __view(_cx: &Cx, parts: &mut PartsWriter<'_>, view: View) {
    parts.push_view(view);
}

#[inline]
pub fn __node(cx: &Cx, parts: &mut PartsWriter<'_>, node: impl NodeViewParts) {
    parts.in_context(HtmlContext::Text, |parts| node.into_view_parts(cx, parts));
}

#[inline]
pub fn __element_name(
    cx: &Cx,
    parts: &mut PartsWriter<'_>,
    element_name: impl ElementNameViewParts,
) {
    parts.in_context(HtmlContext::ElementName, |parts| {
        element_name.into_view_parts(cx, parts);
    });
}

#[inline]
pub fn __attribute_key(
    cx: &Cx,
    parts: &mut PartsWriter<'_>,
    attribute_key: impl AttributeKeyViewParts,
) {
    parts.in_context(HtmlContext::AttributeKey, |parts| {
        attribute_key.into_view_parts(cx, parts);
    });
}

#[inline]
pub fn __attribute_value(
    cx: &Cx,
    parts: &mut PartsWriter<'_>,
    attribute_value: impl AttributeValueViewParts,
) {
    parts.in_context(HtmlContext::AttributeValue, |parts| {
        attribute_value.into_view_parts(cx, parts);
    });
}

#[inline]
pub fn __attribute(
    cx: &Cx,
    parts: &mut PartsWriter<'_>,
    (key, value): (impl AttributeKeyViewParts, impl AttributeValueViewParts),
) {
    __attributes(cx, parts, Attribute::new(key, value));
}

#[inline]
pub fn __attribute_unescaped(
    cx: &Cx,
    parts: &mut PartsWriter<'_>,
    (key, value): (&'static str, impl AttributeValueViewParts),
) {
    __attributes(
        cx,
        parts,
        Attribute::new(Unescaped::new_unchecked(key), value),
    );
}

#[inline]
pub fn __attributes(cx: &Cx, parts: &mut PartsWriter<'_>, attributes: impl AttributeViewParts) {
    // Whole-attribute values do their own context transitions between
    // keys and values; the attribute-value context here is the safe
    // default for any text pushed directly.
    parts.in_context(HtmlContext::AttributeValue, |parts| {
        attributes.into_view_parts(cx, parts);
    });
}
