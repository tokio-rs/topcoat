//! The contract between the view macros and this crate's runtime.
//!
//! Everything here exists for the code the `view!` and `attributes!` macros
//! (and out-of-crate macros composing with them) expand to. The expansions
//! reach it through fully qualified paths, so nothing is imported into the
//! user's scope.
//!
//! A template expands to straight-line code that builds its instruction
//! block through a [`Builder`](crate::internal::Builder), opened at the
//! start of the template and closed at its end. Every node position renders
//! through [`NodePosition`](crate::internal::NodePosition): a value is
//! pushed into the block where it sits, and a view reserves the position
//! and is collected as a [`Pending`](crate::internal::Pending) to drive
//! later. The block and the pendings make up the
//! [`TemplateView`](crate::internal::TemplateView), which fills every
//! reserved position before it reports the block as its content and then
//! streams the swaps of the live ones.
//!
//! Around that, [`ThenView`](crate::internal::ThenView) adapts a
//! component's render future into a view,
//! [`LiveView`](crate::internal::LiveView) backs a `live!` region,
//! [`MoveView`](crate::internal::MoveView) owns a template's captured
//! environment, [`Capture`](crate::internal::Capture) moves a branch's or
//! iteration's bindings into a nested template, and
//! [`ScopeView`](crate::internal::ScopeView) owns the buffer of the build
//! when the template is its outermost view. A body inside a `MoveView` or
//! `live!` region awaits [`drive`](crate::internal::drive) to poll a view
//! in place.

mod builder;
mod capture;
mod drive;
mod live_view;
mod move_view;
mod node_position;
mod pending;
mod scope_view;
mod template_view;
mod then_view;

pub use builder::*;
pub use capture::*;
pub use drive::*;
pub use live_view::*;
pub use move_view::*;
pub use node_position::*;
pub use pending::*;
pub use scope_view::*;
pub use template_view::*;
pub use then_view::*;
