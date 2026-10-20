#![cfg_attr(docsrs, feature(doc_cfg))]

//! Localization for Topcoat.

mod direction;
mod error;
mod extensions;
#[cfg(feature = "fluent")]
mod fluent;
mod langid;
mod locale;
#[cfg(feature = "http")]
mod request;
#[cfg(feature = "router")]
mod router;
mod supported;
#[cfg(feature = "view")]
mod view;

pub use direction::*;
pub use error::*;
pub use extensions::*;
#[cfg(feature = "fluent")]
pub use fluent::*;
pub use langid::*;
pub use locale::*;
#[cfg(feature = "http")]
pub use request::*;
#[cfg(feature = "router")]
pub use router::*;
pub use supported::*;
