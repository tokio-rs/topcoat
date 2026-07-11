//! Implementation details exposed only for this crate's macros.
//!
//! Nothing here is part of the public API; it exists so macros like
//! [`register_font!`](crate::register_font) can reach dependencies through
//! `$crate`.

#[cfg(feature = "discover")]
pub use inventory;
