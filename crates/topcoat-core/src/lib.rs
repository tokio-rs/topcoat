//! Foundations shared by the Topcoat crates: the [`Error`](error::Error) and
//! [`Result`](error::Result) types, the request context [`Cx`](context::Cx),
//! and per-request memoization.
//!
//! Use this crate through the `topcoat` facade, which re-exports its public
//! items and hosts the guides.

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod abort;
pub mod base_url;
#[cfg(feature = "build")]
pub mod cache;
pub mod context;
pub mod cursor;
pub mod error;
pub mod fnv1a;
pub mod identity;
pub mod internal;
pub mod memoize;
pub mod url_form;
