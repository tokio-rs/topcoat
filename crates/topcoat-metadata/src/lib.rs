#![cfg_attr(docsrs, feature(doc_cfg))]

mod field;
#[cfg(feature = "view")]
mod head;
mod metadata;
mod page_metadata;
#[cfg(feature = "view")]
pub mod tag;

pub use field::*;
#[cfg(feature = "view")]
pub use head::*;
pub use metadata::*;
pub use page_metadata::*;
