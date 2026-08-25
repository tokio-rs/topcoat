#![cfg_attr(docsrs, feature(doc_cfg))]

mod buffer;
mod child;
mod component;
mod css;
mod format;
mod html;
pub mod identity;
mod props;
mod string;
pub mod svg;
mod view;

pub use buffer::*;
pub use child::*;
pub use component::*;
pub use css::*;
pub use format::*;
pub use html::*;
pub use props::*;
pub use string::*;
pub use view::*;

/// Macro helpers to shorten the generated source code.
#[doc(hidden)]
pub mod internal;
