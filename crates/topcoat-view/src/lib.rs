#![cfg_attr(docsrs, feature(doc_cfg))]

mod attribute;
mod buffer;
mod class;
mod component;
mod element;
mod escape;
mod format;
mod length;
mod node;
mod part;
mod props;
pub mod svg;
mod unescaped;
mod view;

pub use attribute::*;
pub use class::*;
pub use component::*;
pub use element::*;
pub use escape::*;
pub use format::*;
pub use length::*;
pub use node::*;
pub use part::*;
pub use props::*;
pub use unescaped::*;
pub use view::*;

/// Macro helpers to shorten the generated source code.
#[doc(hidden)]
pub mod internal;
