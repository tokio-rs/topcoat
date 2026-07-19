#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "view")]
mod component;
mod css;
mod display;
mod face;
mod font;
#[cfg(feature = "fontsource")]
pub mod fontsource;
mod format;
#[doc(hidden)]
pub mod internal;
mod resolver;
#[cfg(feature = "router")]
mod router;
mod source;
mod style;
mod tech;
mod unicode;
#[cfg(feature = "view")]
mod view;
mod weight;

#[cfg(feature = "view")]
pub use component::*;
pub(crate) use css::CssString;
pub use display::*;
pub use face::*;
pub use font::*;
pub use format::*;
pub use resolver::*;
#[cfg(feature = "router")]
pub use router::*;
pub use source::*;
pub use style::*;
pub use tech::*;
pub use unicode::*;
pub use weight::*;
