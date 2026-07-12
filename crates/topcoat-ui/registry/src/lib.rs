//! The default Topcoat UI component registry.
//!
//! This crate is the built-in registry that backs `topcoat ui`: its
//! `registry.toml` names the themes and component sources that `topcoat ui add`
//! copies into a project. It is referenced by its crate name
//! ([`topcoat_ui::DEFAULT_REGISTRY_CRATE`]) and, like any registry, must be a
//! direct dependency of the consuming project.
//!
//! The component sources double as this crate's own modules (see [`components`]),
//! so they compile as part of `topcoat-ui-registry`, before being copied
//! verbatim into a project.
//!
//! [`topcoat_ui::DEFAULT_REGISTRY_CRATE`]: https://docs.rs/topcoat-ui

pub mod components;
