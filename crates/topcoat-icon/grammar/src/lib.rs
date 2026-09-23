//! Parsing and code generation for the Topcoat icon macros.
//!
//! This crate is only used at compile time. Use the macros through
//! `topcoat::icon::iconify`.

#![cfg_attr(docsrs, feature(doc_cfg))]

/// The `include!` and `iconify_icon!` macros.
#[cfg(feature = "iconify")]
pub mod iconify;
