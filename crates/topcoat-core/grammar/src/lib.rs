//! Compile-time support for Topcoat's procedural macros: the `#[memoize]`
//! code generator, helpers shared by the macro crates, and, behind the
//! `pretty` feature, the pretty-printer that `topcoat fmt` uses to format
//! macro bodies.
//!
//! This crate is only used by Topcoat's macro crates and the `topcoat` CLI.

#![cfg_attr(docsrs, feature(doc_cfg))]

/// Parsing and code generation for the `#[memoize]` attribute.
pub mod memoize;
/// The [`ParseOption`] trait for parsing optional syntax.
pub mod parse_option;
pub mod paths;
/// The pretty-printer for macro bodies.
#[cfg(feature = "pretty")]
pub mod pretty;
/// The [`QuoteOption`] wrapper for quoting an `Option` as an `Option`.
pub mod quote_option;

pub use parse_option::*;
pub use quote_option::*;
