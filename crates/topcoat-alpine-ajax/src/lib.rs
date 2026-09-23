//! [Alpine AJAX](https://alpine-ajax.js.org) support for Topcoat.
//!
//! Alpine AJAX sends two request headers and has no response headers. The
//! functions [`ajax_request`], [`ajax_targets`], and [`ajax_target`] read the
//! [Alpine AJAX request headers](https://alpine-ajax.js.org/reference/) from a
//! `cx: &Cx`. Everything else, like merging (`x-merge`), targets
//! (`x-target`), and events (`ajax:*`), is set up on the client.
//!
//! The raw header names are available as constants in the [`header`] module.

pub mod header;

mod request;

pub use request::*;
