//! Parsers that accept code while it is being typed.
//!
//! Each type stands in for the `syn` type of the same name. It keeps the
//! tokens it does not need to inspect as they are, so incomplete code there
//! does not stop the macro from expanding.

mod expr;

pub use expr::*;
