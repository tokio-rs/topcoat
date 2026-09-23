//! Icon sets from the [Iconify](https://iconify.design/) catalog.
//!
//! A build script stages icon sets with [`BuildConfig`], and the
//! `iconify::include!` and `iconify::iconify_icon!` macros turn the staged icons
//! into [`IconData`](crate::IconData) constants at compile time.

#[cfg(feature = "iconify-build")]
mod config;
#[cfg(feature = "iconify-build")]
mod error;
mod set;

#[cfg(feature = "iconify-build")]
pub use config::*;
#[cfg(feature = "iconify-build")]
pub use error::*;
pub use set::*;
