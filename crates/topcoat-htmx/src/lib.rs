#![cfg_attr(docsrs, feature(doc_cfg))]
//! [htmx](https://htmx.org) support for Topcoat.
//!
//! Read request headers from `&Cx` and set response headers by placing header
//! values before the body in a response tuple.
//!
//! The raw header names are available as constants in the [`header`] module.

pub mod header;

mod location;
mod request;
mod response;
mod swap;
mod trigger;

pub use location::*;
pub use request::*;
pub use response::*;
pub use swap::*;
pub use trigger::*;
