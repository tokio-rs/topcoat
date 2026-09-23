//! [Tailwind CSS](https://tailwindcss.com) support for Topcoat: a build
//! script wrapper around the standalone Tailwind CLI, and the
//! [`stylesheet!`] macro that links the generated CSS as an asset.
//!
//! Use this crate through `topcoat::tailwind`, which also hosts the guide.

#![cfg_attr(docsrs, feature(doc_cfg))]

mod stylesheet;

#[cfg(feature = "build")]
mod build;

#[cfg(feature = "build")]
pub use build::*;
