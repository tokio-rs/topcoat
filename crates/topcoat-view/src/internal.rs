//! The contract between the view macros and this crate's runtime.
//!
//! Everything here exists for the code the `view!` and `attributes!` macros
//! (and out-of-crate macros composing with them) expand to. The expansions
//! reach it through fully qualified paths, so nothing is imported into the
//! user's scope.
//!
//! A template expands to a composition of the [`View`](crate::View)
//! combinators defined here: a [`JoinView`] drives the template's dynamic
//! node positions concurrently and builds its instruction block from their
//! contents, [`NodeView`] wraps a position's value, [`ThenView`] adapts a
//! future resolving to a view, [`EitherView`] unifies branch types, a
//! [`LoopView`] joins a `for` body's iterations, [`LiveView`] backs a
//! `live!` region, [`MoveView`] owns a template's captured environment, and
//! [`LazyView`] builds a view from the context it is first polled with.
//! The [`Builder`] is the handle a burst pushes its block's parts through,
//! sealed with the HTML context of the position they fill.

mod builder;
mod capture;
mod cx_view;
mod either_view;
mod join_view;
mod lazy_view;
mod live_view;
mod loop_view;
mod move_view;
mod node_view;
mod then_view;

pub use builder::*;
pub use capture::*;
pub use cx_view::*;
pub use either_view::*;
pub use join_view::*;
pub use lazy_view::*;
pub use live_view::*;
pub use loop_view::*;
pub use move_view::*;
pub use node_view::*;
pub use then_view::*;

use crate::{HtmlContext, PartsWriter, ViewBuffer, ViewHandle};

/// Builds a self-contained view handle in one synchronous burst, pushing its
/// parts through the writer handed to `f`.
///
/// Out-of-crate macro expansions use this for values built outside any
/// template, like the runtime's JavaScript views. The handle owns its
/// buffer, so it can be spliced into any view later.
pub fn build_sync(f: impl FnOnce(&mut PartsWriter<'_>)) -> ViewHandle {
    let mut buffer = ViewBuffer::new();
    let handle = buffer.write_block(f);
    handle.seal(buffer)
}

/// Runs `f` with the writer switched to `context`, restoring the previous
/// context afterwards.
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
