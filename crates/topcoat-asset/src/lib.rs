#![cfg_attr(docsrs, feature(doc_cfg))]

mod asset;
mod bundle;
mod error;
mod manifest;
mod options;
mod resolver;
mod source;

pub use asset::*;
pub use bundle::*;
pub use error::*;
pub use manifest::*;
pub use options::*;
pub use resolver::*;
pub use source::*;

pub use topcoat_core::cursor::{ConstReader, ConstWriter};

#[cfg(feature = "bundler")]
mod bundler;

#[cfg(feature = "bundler")]
pub use bundler::*;

#[cfg(feature = "router")]
mod router;

#[cfg(feature = "router")]
pub use router::*;

#[cfg(feature = "view")]
mod view;
