#![cfg_attr(docsrs, feature(doc_cfg))]

#[cfg(feature = "view")]
mod head;
mod metadata;
mod page_metadata;
#[cfg(feature = "view")]
pub mod tag;
mod text;
mod url;

#[cfg(feature = "view")]
pub use head::*;
pub use metadata::*;
pub use page_metadata::*;
pub use text::*;
pub use url::*;
