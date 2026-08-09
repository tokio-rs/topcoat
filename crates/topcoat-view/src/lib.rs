#![cfg_attr(docsrs, feature(doc_cfg))]

mod buffer;
mod component;
mod css;
mod format;
mod html;
pub mod identity;
mod part;
mod props;
mod static_str;
pub mod svg;
mod unescaped;
mod view;

pub use component::*;
pub use css::*;
pub use format::*;
pub use html::*;
pub use part::*;
pub use props::*;
pub use static_str::*;
pub use unescaped::*;
pub use view::*;

/// Macro helpers to shorten the generated source code.
#[doc(hidden)]
pub mod internal;
