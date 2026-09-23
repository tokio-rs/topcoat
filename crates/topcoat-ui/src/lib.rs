#![cfg_attr(docsrs, feature(doc_cfg))]
//! Component registries and the `topcoat ui` workflow.
//!
//! Like [shadcn/ui](https://ui.shadcn.com), Topcoat UI components are not used
//! as a library dependency. `topcoat ui add <name>` copies the source of a
//! component into your project, where you can change it freely.
//!
//! This crate reads the *registries* that the components come from. A
//! registry is a Cargo crate with a `[package.metadata.topcoat-ui]` key. The
//! key points at a directory that holds a hand-written `registry.toml`
//! manifest next to the component sources. A project refers to a registry by
//! its crate name and must depend on it, so the component sources are on
//! disk at the version the project uses. The CLI reads registries when it
//! runs, so it does not need to be rebuilt when their components change.
//!
//! The version of each component is a hash of its source, computed with
//! [`content_hash`]. `registry.toml` records no hashes. When you add a
//! component, its hash is recorded in the project's install state. Comparing
//! it with a fresh hash of the registry source shows whether an update is
//! available.
//!
//! The [`manage`] module implements the `topcoat ui` commands.

pub mod manage;
mod registry;

pub use registry::*;
