#![cfg_attr(docsrs, feature(doc_cfg))]

mod buffer;
mod component;
mod css;
mod format;
mod html;
pub mod identity;
mod part;
pub mod pass;
mod props;
mod string;
pub mod svg;
mod view;

pub use component::*;
pub use css::*;
pub use format::*;
pub use html::*;
pub use part::*;
pub use pass::View;
pub use props::*;
pub use string::*;
pub use view::*;

/// Macro helpers to shorten the generated source code.
#[doc(hidden)]
pub mod internal;
