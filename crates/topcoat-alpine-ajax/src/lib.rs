//! [Alpine AJAX](https://alpine-ajax.js.org) support for Topcoat.
//!
//! Read request headers from `&Cx` to identify AJAX requests and their target
//! elements. Configure page updates in the HTML using Alpine AJAX attributes.
//!
//! The raw header names are available as constants in the [`header`] module.

pub mod header;

mod request;

pub use request::*;
