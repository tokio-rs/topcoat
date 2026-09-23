//! [Datastar](https://data-star.dev) support for Topcoat.
//!
//! In Datastar, `data-*` attributes bind signals in the browser, actions like
//! `@get` and `@post` call the server, and the server answers with events that
//! patch elements and signals into the page. This crate is the server side:
//!
//! - [`Signals`] extracts the signals that a Datastar action sends with a request, and
//!   [`datastar_request`] checks whether a request came from Datastar.
//! - [`PatchElements`], [`PatchSignals`], and [`ExecuteScript`] are the events that Datastar
//!   applies. Each converts into a server-sent [`Event`](topcoat_router::content::sse::Event) for
//!   an [`Sse`](topcoat_router::content::sse::Sse) stream. Returned from a handler on its own, each
//!   responds with a stream of just that event.
//! - Types like [`DatastarSelector`] and [`DatastarMode`] implement
//!   [`IntoResponseParts`](topcoat_router::response::IntoResponseParts). They set the headers that
//!   control how Datastar applies a plain `text/html`, `application/json`, or `text/javascript`
//!   response.
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
