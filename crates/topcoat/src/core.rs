//! Lower-level building blocks shared by the other modules.
//!
//! Most applications do not need this module. It holds the error types
//! returned by failed downcasts and the [`identity`] module, which derives
//! stable identities from source locations and keys.

pub use topcoat_core::{
    error::{DowncastError, DowncastFailure},
    identity,
};
