//! Parsing and code generation for the runtime macros: `expr!`,
//! `#[procedure]`, and `#[shard]`.
//!
//! This crate is only used at compile time, by `topcoat-runtime-macro`.
#![cfg_attr(docsrs, feature(doc_cfg))]

/// The `expr!` macro, which compiles an expression to Rust and JavaScript.
pub mod expr;
/// The `#[procedure]` attribute macro.
pub mod procedure;
/// The `#[shard]` attribute macro.
pub mod shard;
