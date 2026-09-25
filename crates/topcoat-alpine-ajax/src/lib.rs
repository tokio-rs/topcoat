//! [Alpine AJAX](https://alpine-ajax.js.org) support for Topcoat.
//!
//! Use [`ajax_request`] to recognize Alpine AJAX requests and [`ajax_targets`]
//! to read their target element IDs from `cx: &Cx`. Return matching HTML
//! fragments from your handlers.
//!
//! Raw header names are available in [`header`].

pub mod header;

mod request;

pub use request::*;
