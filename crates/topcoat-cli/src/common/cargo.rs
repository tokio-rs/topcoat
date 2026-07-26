//! Running cargo: metadata queries and application builds.
//!
//! [`Metadata`] wraps `cargo metadata` queries. [`BuildOpts::build`] runs
//! `cargo build` and interprets its output, with the plumbing split by
//! concern:
//!
//! - [`messages`]: the JSON message stream cargo writes to stdout -- rustc's diagnostics and the
//!   artifacts of every compiled crate.
//! - [`artifacts`]: picking the final linked outputs out of those artifacts.
//! - [`stderr`]: capturing cargo's stderr and extracting its error report.
//! - [`progress`]: scanning the stderr stream for build progress.

mod artifacts;
mod build;
mod messages;
mod metadata;
mod progress;
mod stderr;

pub use build::*;
pub use metadata::*;
