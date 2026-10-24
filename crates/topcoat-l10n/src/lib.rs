#![cfg_attr(docsrs, feature(doc_cfg))]

//! Locale selection and translation support for Topcoat.

#[cfg(feature = "datetime")]
mod datetime;
mod direction;
mod error;
mod extensions;
#[cfg(feature = "fluent")]
mod fluent;
mod langid;
mod locale;
mod lookup;
mod negotiate;
#[cfg(feature = "number")]
mod number;
#[cfg(feature = "http")]
mod request;
#[cfg(feature = "serde")]
mod serde;
#[cfg(feature = "view")]
mod view;

#[cfg(feature = "datetime")]
pub use datetime::*;
pub use direction::*;
pub use error::*;
pub use extensions::*;
#[cfg(feature = "fluent")]
pub use fluent::*;
pub use langid::*;
pub use locale::*;
pub use lookup::*;
pub use negotiate::*;
#[cfg(feature = "number")]
pub use number::*;
#[cfg(feature = "http")]
pub use request::*;
