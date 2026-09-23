//! Parsing and code generation for the Topcoat font macros.
//!
//! This crate is only used at compile time. Use the macros through `topcoat::font`.

#![cfg_attr(docsrs, feature(doc_cfg))]

/// The `font!` macro.
pub mod font;
/// The `font_face!` macro.
pub mod font_face;

/// The `fontsource_font!` and `fontsource_font_face!` macros.
#[cfg(feature = "fontsource")]
pub mod fontsource;
