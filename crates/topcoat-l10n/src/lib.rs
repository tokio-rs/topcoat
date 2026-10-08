#![cfg_attr(docsrs, feature(doc_cfg))]

//! Localization for Topcoat.

mod error;
mod extensions;
mod langid;
mod locale;

pub use error::*;
pub use extensions::*;
pub use langid::*;
pub use locale::*;
