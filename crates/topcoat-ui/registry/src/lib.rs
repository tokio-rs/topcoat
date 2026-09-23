#![cfg_attr(docsrs, feature(doc_cfg))]
//! The built-in Topcoat UI component registry.
//!
//! This crate holds the themes and components that `topcoat ui` installs by
//! default. Its `registry.toml` names each theme and component source file,
//! and `topcoat ui add` copies those files into a project unchanged. The `ui`
//! feature of `topcoat` depends on this crate, and `topcoat ui` refers to it
//! by the name `topcoat`.
//!
//! The component sources are also modules of this crate, so they are
//! type-checked against `topcoat`. They only compile in tests (with the
//! `stage-icons` feature), because `topcoat` can only be a dev-dependency here:
//! a normal dependency would form a cycle with the `topcoat` crate, which
//! depends on this one.

#[cfg(all(test, feature = "stage-icons"))]
pub mod components;
