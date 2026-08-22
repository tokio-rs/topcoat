//! The contract between the view macros and this crate's runtime.
//!
//! Everything here exists for the code the `view!` and `attributes!` macros
//! (and out-of-crate macros composing with them) expand to. The expansions
//! reach it through fully qualified paths, so nothing is imported into the
//! user's scope.
//!
//! The module has two concerns. The build entry points (`build`,
//! `build_sync`, `block`, `reserve`) manage who owns the instruction
//! buffer and where a view's instruction block starts and ends. The
//! `Builder` is the emission handle a `block` hands out: its methods
//! push the block's parts, sealed with the HTML context of the position
//! they fill.
//!
//! Two rules keep the buffer's blocks executable. A block's instructions
//! must land contiguously, so everything pushed through a `Builder`
//! happens in one synchronous burst with no `await` in between. A
//! placeholder must form a block of its own, so `reserve` is called
//! between blocks, never while one is building.

use futures_util::FutureExt;
pub use futures_util::{
    future::{Either, try_join_all},
    try_join,
};
use topcoat_core::{context::Cx, error::Result};

pub use crate::buffer::ViewSlot;
use crate::{
    Attribute, AttributeKeyViewParts, AttributeValueViewParts, AttributeViewParts,
    ElementNameViewParts, HtmlContext, NodeViewParts, PartsWriter, Unescaped, View,
    buffer::{ViewBuffer, ViewBufferScope},
};

/// Builds a top-level `view!` invocation, deciding who owns the buffer.
///
/// When an enclosing invocation is already building on this task, this is a
/// nested build: `fut` appends to the enclosing buffer and the returned view
/// stays a handle into it. Otherwise this invocation is the root: a fresh
/// buffer is installed while `fut` polls, and the returned view takes
/// ownership of it.
pub fn build(
    fut: impl Future<Output = Result<View>> + Send,
) -> impl Future<Output = Result<View>> + Send {
    ViewBufferScope::scope(fut).map(|(view, buffer)| Ok(view?.seal(buffer)))
}

/// Builds a view in one synchronous burst, in the enclosing invocation's
/// buffer when one is building on this task and in a buffer of its own
/// otherwise.
///
/// The synchronous counterpart of [`build`]: `f` composes the view from
/// blocks, usually a single [`block`] or [`write_block`]. Runtime
/// collections like [`Attributes`](crate::Attributes) capture values
/// through this, so they work standalone as well as inside a `view!`.
pub fn build_sync(f: impl FnOnce() -> View) -> View {
    let (view, buffer) = ViewBufferScope::scope_sync(f);
    view.seal(buffer)
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
pub fn block(cx: &Cx, f: impl FnOnce(&mut Builder<'_, '_, '_>)) -> View {
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
pub fn write_block(f: impl FnOnce(&mut PartsWriter<'_>)) -> View {
    ViewBufferScope::with(|buffer| PartsWriter::block(buffer, f))
}

/// Reserves a slot in the installed buffer for a view that resolves later.
///
/// A component whose children render components of their own passes the
/// placeholder view to its props so it can render concurrently with the
/// children; [`ViewSlot::fill`] redirects the slot once they resolve.
///
/// # Panics
///
/// Panics if no view is building on the current task.
#[must_use]
pub fn reserve() -> (View, ViewSlot) {
    ViewBufferScope::with(ViewBuffer::reserve_view)
}

/// Wraps an already-built view in a ready future.
///
/// Splices a view without components into a joined position, like the branch
/// of an `if` whose other branch renders components.
pub fn ready(view: View) -> futures_util::future::Ready<Result<View>> {
    futures_util::future::ready(Ok(view))
}

/// Moves a control-flow body's pattern bindings into its render future.
///
/// A joined branch or iteration body expands to a future that borrows its
/// environment, while the values its pattern binds die with the branch or
/// iteration that produced them. The expansion packs those values into this
/// wrapper where they are still alive and takes them back inside the future,
/// which then owns them for as long as it lives.
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
pub fn view(parts: &mut PartsWriter<'_>, view: View) {
    parts.push_view(view);
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
    pub fn view(&mut self, view: View) {
        self.parts.push_view(view);
    }
}
