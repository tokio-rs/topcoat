//! The streaming pass runtime.
//!
//! A streaming page renders in passes: the first pass produces the full
//! document with placeholders for deferred data, and every later pass
//! re-renders the components whose renders run again, so the driver can ship
//! what changed. A component is one future: its body runs once, then its
//! render loop renders once per pass and suspends at [`pass_boundary`]
//! between passes. Children live in the parent's [`Children`] table, output
//! travels through per component slots assembled by marker substitution, and
//! deferred loads register through [`defer`] and resolve between passes.
//!
//! This module is the hand written target of the `view!` and `#[component]`
//! macros; the design and its validation live in the repository's
//! `DESIGN.md` and `prototype/streaming-model`.

mod children;
mod defer;
mod driver;
mod render;
mod scope;

pub use children::*;
pub use defer::*;
pub use driver::*;
pub use render::*;
