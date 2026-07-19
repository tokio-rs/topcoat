#![cfg_attr(docsrs, feature(doc_cfg))]
//! The default Topcoat UI component registry.
//!
//! This crate is the built-in registry that backs `topcoat ui`: its
//! `registry.toml` names the themes and component sources that `topcoat ui add`
//! copies into a project. The `topcoat` facade pulls it in under its `ui`
//! feature, and `topcoat ui` refers to it by the alias `topcoat`
//! ([`topcoat_ui::DEFAULT_REGISTRY_CRATE`]).
//!
//! The component sources double as this crate's own modules (see the
//! `components` module), so they are type-checked against `topcoat` while they
//! are developed here, before being copied verbatim into a project. They compile
//! under `cfg(test)` because `topcoat` is a dev-dependency: it cannot be a normal
//! one without forming a dependency cycle with the facade crate that pulls this
//! registry in.
//!
//! [`topcoat_ui::DEFAULT_REGISTRY_CRATE`]: https://docs.rs/topcoat-ui

#[cfg(all(test, feature = "stage-icons"))]
pub mod components;
