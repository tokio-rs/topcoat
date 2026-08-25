//! The contract between the view macros and this crate's runtime.
//!
//! Everything here exists for the code the `view!` and `attributes!` macros
//! (and out-of-crate macros composing with them) expand to. The expansions
//! reach it through fully qualified paths, so nothing is imported into the
//! user's scope.
//!
//! A template expands to a [`ViewStream`] whose body drives its dynamic
//! node positions concurrently: each position becomes a unit future
//! ([`unit_future`]) in a [`Join`], the joined content lands in the body's
//! own instruction block ([`block`]), and the block is yielded as the
//! template's content chunk ([`emit_content`]). The [`Builder`] is the
//! emission handle a `block` hands out: its methods push the block's parts,
//! sealed with the HTML context of the position they fill.

use std::{marker::PhantomData, ops::DerefMut};

pub use futures_core::Stream;
use topcoat_core::{context::Cx, error::Result};

mod either_view;
mod join_view;
mod live_view;
mod loop_view;
mod node_view;
mod then_view;

pub use either_view::*;
pub use join_view::*;
pub use live_view::*;
pub use loop_view::*;
pub use node_view::*;
pub use then_view::*;
use crate::{
    Attribute, AttributeKeyViewParts, AttributeValueViewParts, AttributeViewParts,
    ElementNameViewParts, HtmlContext, NodeViewPartsStream, NodeWriter, PartsWriter, Unescaped,
    View, ViewChunk, ViewHandle, buffer::ViewBufferScope, html::forward_view,
};

/// Returns the future driving `value` at a node position: a unit of the
/// template's [`Join`].
pub fn unit_future<'cx, T>(value: T, cx: &'cx Cx) -> impl Future<Output = Result<()>> + Send + 'cx
where
    T: NodeViewPartsStream + 'cx,
{
    value.into_view_parts_stream(cx, NodeWriter::new())
}

/// A component's render future as a node value, joined like any other
/// dynamic position.
///
/// The future resolves to the component's view, whose chunks then stream
/// through the position.
pub struct Render<F, V> {
    future: F,
    // Names the view type, so it participates in the lifetime bounds of the
    // trait method's returned future.
    view: PhantomData<fn() -> V>,
}

impl<F, V> Render<F, V>
where
    F: Future<Output = Result<V>>,
{
    pub fn new(future: F) -> Self {
        Self {
            future,
            view: PhantomData,
        }
    }
}

impl<F, V> NodeViewPartsStream for Render<F, V>
where
    F: Future<Output = Result<V>> + Send,
    V: View,
{
    const MULTI: bool = false;

    async fn into_view_parts_stream<'cx>(self, _cx: &'cx Cx, mut writer: NodeWriter) -> Result<()>
    where
        Self: 'cx,
    {
        forward_view(self.future.await?, &mut writer).await
    }
}

/// Moves a control-flow body's pattern bindings into its nested stream.
///
/// A branch or iteration body expands to a stream that borrows its
/// environment, while the values its pattern binds die with the branch or
/// iteration that produced them. The expansion packs those values into this
/// wrapper where they are still alive and takes them back inside the
/// stream's body, which then owns them for as long as it lives.
///
/// The wrapper is deliberately not `Copy`, and [`take`](Self::take) consumes
/// it whole: a by-value use of a whole non-`Copy` place is captured by value
/// even in a non-`move` async block. Reading the contents through the field
/// instead would let capture analysis narrow to the possibly `Copy` values
/// inside and downgrade the capture to a borrow, which would not live long
/// enough.
pub struct Capture<T>(pub T);

impl<T> Capture<T> {
    /// Returns the packed bindings, consuming the wrapper.
    pub fn take(self) -> T {
        self.0
    }
}

/// Appends a view's instruction block in one synchronous burst, pushing its
/// parts through the [`Builder`] handed to `f`.
///
/// Records the entry address in the installed buffer, runs `f`, and
/// terminates the block with a return instruction. The returned view handle
/// carries the builder's accumulated size hint. `f` must not build other
/// views; nested views are built first and spliced into the block with
/// [`Builder::view`].
///
/// # Panics
///
/// Panics if no view is building on the current task.
pub fn block(cx: &Cx, f: impl FnOnce(&mut Builder<'_, '_, '_>)) -> ViewHandle {
    write_block(|parts| f(&mut Builder { cx, parts }))
}

/// Appends a view's instruction block in one synchronous burst, pushing its
/// parts through the writer handed to `f`.
///
/// The writer counterpart of [`block`], for compositions that push through
/// the writer directly instead of a [`Builder`], like the runtime's
/// JavaScript views.
///
/// # Panics
///
/// Panics if no view is building on the current task.
pub fn write_block(f: impl FnOnce(&mut PartsWriter<'_>)) -> ViewHandle {
    ViewBufferScope::with(|buffer| PartsWriter::block(buffer, f))
}

/// Runs `f` with the writer sealing for a different context, then restores
/// the current context.
///
/// Out-of-crate macro expansions use this for compositions that span more
/// than one position, like the runtime's JavaScript views.
#[inline]
pub fn in_context<R>(
    parts: &mut PartsWriter<'_>,
    context: HtmlContext,
    f: impl FnOnce(&mut PartsWriter<'_>) -> R,
) -> R {
    parts.in_context(context, f)
}

/// Splices an already-built view through a writer.
///
/// The writer counterpart of [`Builder::view`], for out-of-crate macro
/// expansions that compose views inside a `*ViewParts` implementation.
///
/// # Panics
///
/// Panics if the view was built in a different, still building buffer.
#[inline]
pub fn view(parts: &mut PartsWriter<'_>, view: ViewHandle) {
    parts.push_view_handle(view);
}

/// The emission handle of a [`block`]: the request context plus a writer
/// over the installed buffer.
///
/// Each method pushes one of the block's parts, sealing it with the
/// [`HtmlContext`] of the position it fills by dispatching the matching
/// `*ViewParts` trait.
pub struct Builder<'a, 'b, 'c> {
    cx: &'a Cx,
    parts: &'b mut PartsWriter<'c>,
}

impl Builder<'_, '_, '_> {
    /// Appends a literal markup segment, verbatim.
    ///
    /// The segment is passed as `&"..."` so it stays out of the buffer's
    /// constants.
    #[inline]
    pub fn markup(&mut self, s: &'static &'static str) {
        self.parts.push_promoted_str_unescaped(s);
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
