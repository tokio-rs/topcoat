//! Premade UI components for Topcoat applications.
//!
//! In the spirit of [shadcn/ui](https://ui.shadcn.com), components are not
//! consumed as an opaque dependency. Instead, `topcoat ui add <name>` copies a
//! component's source straight into the user's project, where it can be freely
//! modified.
//!
//! This crate models the *registry* that backs that command. A registry is a
//! cargo crate that carries a `[package.metadata.topcoat-ui]` key pointing at a
//! directory holding a `registry.toml` manifest alongside the component source
//! files. A registry is referenced by its crate name and must be a dependency
//! of the consuming project, so its component source is always present locally
//! at the matching version. Registries are read at runtime, so the set of
//! available components can change without rebuilding the CLI.
//!
//! Each component is versioned independently by a hash of its source (see
//! [`content_hash`]). A `registry.toml` records no hashes: it names each
//! component and its source file, and the hash is computed from the registry's
//! current source. The hash is recorded in the project's install state when a
//! component is added, then recomputed from the registry to surface updates.
//!
//! A `registry.toml` is written by hand; there is no generator.

pub mod manage;
mod registry;

pub use registry::*;
