//! Inline SVG icons for Topcoat: the [`IconData`] type, the [`icon`]
//! component that renders it, and an optional integration with the
//! [Iconify](https://iconify.design/) icon catalog.
//!
//! Use this crate through the `topcoat::icon` module.

#![cfg_attr(docsrs, feature(doc_cfg))]

mod component;
mod data;
#[cfg(feature = "iconify")]
pub mod iconify;

pub use component::*;
pub use data::*;
