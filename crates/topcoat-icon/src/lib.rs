#![cfg_attr(docsrs, feature(doc_cfg))]

mod component;
mod data;
#[cfg(feature = "iconify")]
pub mod iconify;

pub use component::*;
pub use data::*;
