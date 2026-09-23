//! Queries workspace metadata and builds applications with Cargo.
//!
//! Use [`Metadata`] to inspect the workspace and [`BuildOpts::build`] to
//! compile a target and locate its output.

mod artifacts;
mod build;
mod messages;
mod metadata;
mod progress;
mod stderr;

pub use build::*;
pub use metadata::*;
