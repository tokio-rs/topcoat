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
//! `live!` region, and [`MoveView`] owns a template's captured environment.
//! The [`Builder`] is the handle a burst pushes its block's parts through,
//! sealed with the HTML context of the position they fill.

mod builder;
mod capture;
mod cx_view;
mod either_view;
mod join_view;
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
pub use live_view::*;
pub use loop_view::*;
pub use move_view::*;
pub use node_view::*;
pub use then_view::*;
