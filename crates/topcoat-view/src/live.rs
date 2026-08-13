//! The live render runtime.
//!
//! A live render does not stop when its view is built: each component's
//! future hands its view over through a [`Fill`] and keeps running until
//! nothing inside it can change anymore. The pieces here are what the view
//! macros compose that behavior from:
//!
//! - [`Reactive`] is the contract between a reactive expression and the view construct consuming
//!   it: a state to render now, and zero or more replacement states later.
//! - [`defer`] wraps a slow future instead of awaiting it, producing the first reactive expression,
//!   [`Defer`], whose states are [`Deferred`].
//! - [`RefreshSet`] collects one frame's live work: fused component invocations, adopted handles,
//!   and reactive node futures.
//! - [`ViewHandle`] is the reactive view handle: a cheap clone of a render in flight, spliced
//!   wherever the view should show.
//! - [`run_node`] drives one reactive construct: render the current state, then race the shown
//!   arm's remaining work against the next state.
//! - [`LiveRender`] owns one whole render and drives it: the router composes the page and its
//!   layouts into one root future and reads the document out of it.

mod handle;
mod node;
mod reactive;
mod refresh_set;
mod render;

pub use handle::*;
pub use node::*;
pub use reactive::*;
pub use refresh_set::*;
pub use render::*;

#[doc(hidden)]
pub mod test_support;
