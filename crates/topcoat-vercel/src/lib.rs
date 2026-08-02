#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc = include_str!("../README.md")]

mod runtime;

pub use runtime::*;

#[cfg(feature = "cli")]
#[doc(hidden)]
pub mod cli;
