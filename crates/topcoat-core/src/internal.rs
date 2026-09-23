//! Items used by code that Topcoat's macros generate. Not part of the public
//! API.

/// Names the success and error types of a [`Result`] type.
pub trait ResultExt {
    /// The success type.
    type T;
    /// The error type.
    type E;
}

impl<T, E> ResultExt for Result<T, E> {
    type T = T;
    type E = E;
}
