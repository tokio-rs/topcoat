//! Parsing and code generation for the Topcoat router macros.
//!
//! This crate is used at compile time by `topcoat-router-macro`. Each module
//! holds the syntax tree for one macro and generates its expansion through
//! [`quote::ToTokens`].

#![cfg_attr(docsrs, feature(doc_cfg))]

/// Syntax shared by several macros: handler paths, handler parameters, and
/// the `error = ...` argument.
pub mod common;
/// The `#[layer]` attribute macro.
pub mod layer;
/// The `#[layout]` attribute macro.
pub mod layout;
/// The HTTP method list of `#[page]` and `#[route]`.
pub mod method;
/// The `not_found!` macro.
pub mod not_found;
/// The `#[page]` attribute macro.
pub mod page;
/// The `path_param!` macro.
pub mod path_param;
/// The `#[query_params]` attribute macro.
pub mod query_params;
/// The `#[route]` attribute macro.
pub mod route;
/// The `segment!` macro.
pub mod segment;
