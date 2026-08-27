//! The contract between the view macros and this crate's runtime.
//!
//! Everything here exists for the code the `view!` and `attributes!` macros
//! (and out-of-crate macros composing with them) expand to. The expansions
//! reach it through fully qualified paths, so nothing is imported into the
//! user's scope.
//!
//! A template expands to a composition of the [`View`](crate::View)
//! combinators defined here: a [`JoinView`](crate::internal::JoinView) drives the template's
//! dynamic node positions concurrently and builds its instruction block from their
//! contents, [`NodeView`](crate::internal::NodeView) wraps a position's value,
//! [`ThenView`](crate::internal::ThenView) adapts a future resolving to a view,
//! [`EitherView`](crate::internal::EitherView) unifies branch types, a
//! [`LoopView`](crate::internal::LoopView) joins a `for` body's iterations,
//! [`LiveView`](crate::internal::LiveView) backs a `live!` region,
//! [`MoveView`](crate::internal::MoveView) owns a template's captured environment, and
//! [`ScopeView`](crate::internal::ScopeView) owns the buffer of the build when the template is
//! its outermost view. A body inside a `MoveView` or `live!` region awaits
//! [`drive`](crate::internal::drive) to poll a view in place.
//! The [`Builder`](crate::internal::Builder) is the handle a burst pushes its block's
//! parts through, sealed with the HTML context of the position they fill.

mod builder;
mod capture;
mod drive;
mod either_view;
mod join_view;
mod live_view;
mod loop_view;
mod move_view;
mod node_view;
mod scope_view;
mod then_view;

pub use builder::*;
pub use capture::*;
pub use drive::*;
pub use either_view::*;
pub use join_view::*;
pub use live_view::*;
pub use loop_view::*;
pub use move_view::*;
pub use node_view::*;
pub use scope_view::*;
pub use then_view::*;
