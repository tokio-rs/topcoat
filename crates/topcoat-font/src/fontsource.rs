//! Font families from the [Fontsource](https://fontsource.org/) catalog.
//!
//! A copy of the catalog is built into this crate, so families, weights,
//! styles, and subsets can be checked at compile time.

mod family;
mod host;
mod style;
mod subset;

pub mod families;

pub use family::*;
pub use host::*;
pub use style::*;
pub use subset::*;
