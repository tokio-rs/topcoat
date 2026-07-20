//! [Alpine AJAX](https://alpine-ajax.js.org) support for Topcoat.
//!
//! Alpine AJAX's wire protocol is much thinner than htmx's: the client sends
//! two request headers and defines no server-to-client response header
//! convention at all. This crate provides read-only accessors for those
//! request headers, built on Topcoat's request context rather than on
//! extractors or middleware:
//!
//! - **Request accessors** ([`ajax_request`], [`ajax_targets`], [`ajax_target`])
//!   read the [Alpine AJAX request headers](https://alpine-ajax.js.org/reference/)
//!   from a `cx: &Cx`.
//!
//! Merge strategies (`x-merge`), navigation (`x-target`), and client-side
//! events (`ajax:*`) are pure client-side concerns configured directly in
//! markup or JS and have no server-side counterpart to wrap.
//!
//! The raw header names are available as constants in the [`header`] module.

pub mod header;

mod request;

pub use request::*;
