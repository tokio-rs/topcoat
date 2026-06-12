//! Premade UI components for Topcoat applications.
//!
//! In the spirit of [shadcn/ui](https://ui.shadcn.com), components are not
//! consumed as an opaque dependency. Instead, `topcoat ui add <name>` copies a
//! component's source straight into the user's project, where it can be freely
//! modified.
//!
//! This crate models the *registry* that backs that command. A registry is just
//! a [`Source`] — a local directory or a remote base URL — containing a
//! `registry.toml` manifest alongside the component source files. Registries are
//! read at runtime, so the set of available components can change without
//! rebuilding the CLI, and projects can point at custom or remote registries.
//!
//! Each component is versioned independently by a hash of its source (see
//! [`content_hash`]). A `registry.toml` records no hashes: it names each
//! component and its source file, and the hash is computed on the fly from the
//! registry's current source. The hash is recorded per component in the
//! project's install state when it is added, then recomputed from the registry
//! to surface updates for individual components.
//!
//! A `registry.toml` is written by hand — there is no generator.

pub mod manage;
mod registry;

pub use registry::{
    Component, DEFAULT_REGISTRY, Dependency, Error, MANIFEST_FILE, Registry, Source, content_hash,
};
