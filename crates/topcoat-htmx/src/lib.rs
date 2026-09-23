#![cfg_attr(docsrs, feature(doc_cfg))]
//! [htmx](https://htmx.org) support for Topcoat.
//!
//! Read request headers from `cx: &Cx` with accessors such as [`hx_request`].
//! Set response headers by placing a responder before the body in a response
//! tuple. Responders implement
//! [`IntoResponseParts`](topcoat_router::response::IntoResponseParts).
//!
//! Raw header names are available in [`header`].

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
