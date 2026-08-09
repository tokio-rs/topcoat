//! Invocation identity: a stable id for each invocation in a tree of
//! nested calls, derived from the chain of call sites leading down to it.
//!
//! `view!` derives an [`Identity`] for every component invocation, so a
//! component keeps the same id from one render to the next. A component
//! invoked in a `for` body passes a `key:` argument to give each
//! repetition its own identity; without one the identity is ambiguous and
//! errors when consumed. Consumers read the identity of the running
//! invocation with [`Identity::current`].

pub use topcoat_core::identity::*;
