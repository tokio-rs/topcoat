#![cfg_attr(docsrs, feature(doc_cfg))]
//! [htmx](https://htmx.org) support for Topcoat.
//!
//! This crate has two parts:
//!
//! - Functions like [`hx_request`], [`hx_target`], and [`hx_trigger`] read the [htmx request headers](https://htmx.org/reference/#request_headers)
//!   from a `cx: &Cx`.
//! - Types like [`HxRedirect`], [`HxRetarget`], and [`HxResponseTrigger`] set the [htmx response headers](https://htmx.org/reference/#response_headers).
//!   They implement [`IntoResponseParts`](topcoat_router::response::IntoResponseParts), so you
//!   place them before the body in a handler's response tuple.
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
