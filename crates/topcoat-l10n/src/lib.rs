#![cfg_attr(docsrs, feature(doc_cfg))]

//! Localization for Topcoat.

mod error;
mod extensions;
mod langid;
mod locale;
#[cfg(feature = "router")]
mod request;
#[cfg(feature = "router")]
mod router;
mod supported;

pub use error::*;
pub use extensions::*;
pub use langid::*;
pub use locale::*;
#[cfg(feature = "router")]
pub use request::*;
#[cfg(feature = "router")]
pub use router::*;
pub use supported::*;
