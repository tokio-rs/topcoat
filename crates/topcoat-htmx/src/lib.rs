#![cfg_attr(docsrs, feature(doc_cfg))]
//! [htmx](https://htmx.org) support for Topcoat.
//!
//! This crate provides two halves of htmx integration, both built on Topcoat's
//! request context and response conventions rather than on extractors and
//! middleware:
//!
//! - **Request accessors** ([`hx_request`], [`hx_boosted`], [`hx_target`], ...)
//!   read the [htmx request headers](https://htmx.org/reference/#request_headers)
//!   from a `cx: &Cx`.
//! - **Responders** ([`HxRedirect`], [`HxRefresh`], [`HxRetarget`],
//!   [`HxResponseTrigger`], ...) implement
//!   [`IntoResponseParts`](topcoat_router::IntoResponseParts), so they
//!   can be placed before the body in a handler's response tuple to set the
//!   corresponding [htmx response headers](https://htmx.org/reference/#response_headers).
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
