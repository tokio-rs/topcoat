#![cfg_attr(docsrs, feature(doc_cfg))]
//! Premade UI components for Topcoat applications.
//!
//! Run `topcoat ui add <name>` to copy a component into your project, then edit
//! its source to suit your app.
//!
//! This crate loads component registries and manages installed components.
//! A registry is a Cargo crate with `[package.metadata.topcoat-ui]` metadata
//! pointing to a directory containing `registry.toml` and component sources.
//!
//! Components are versioned by a hash of their source. The install state saves
//! that hash so later comparisons with the registry can identify updates.

pub mod manage;
mod registry;

pub use registry::*;
