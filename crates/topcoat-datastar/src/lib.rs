//! [Datastar](https://data-star.dev) support for Topcoat.
//!
//! Read signals from requests and return patches to update the page. Patches
//! work as individual responses or as events in a server-sent event stream.
//! Response header types also control updates from plain responses.
//!
//! The raw header names are available as constants in the [`header`] module.

pub mod header;

mod common;
mod execute_script;
mod patch_elements;
mod patch_mode;
mod patch_signals;
mod request;
mod response;
mod signals;

pub use execute_script::*;
pub use patch_elements::*;
pub use patch_mode::*;
pub use patch_signals::*;
pub use request::*;
pub use response::*;
pub use signals::*;
