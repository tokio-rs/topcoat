#![doc = include_str!("../docs/view.md")]

// Each builtin component's module is named apart from its file: a module
// named `suspense` would shadow the re-exported `suspense` component.
#[path = "view/error_boundary.rs"]
mod error_boundary_component;
#[path = "view/suspense.rs"]
mod suspense_component;

pub use error_boundary_component::*;
pub use suspense_component::*;
pub use topcoat_view::*;
pub use topcoat_view_macro::*;
