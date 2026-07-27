//! [Datastar](https://data-star.dev) support for Topcoat.
//!
//! Datastar drives page updates from the backend: `data-*` attributes bind
//! reactive signals in the browser, actions like `@get` and `@post` call the
//! server, and the server answers with events that patch elements and signals
//! into the page. This crate provides the backend half, built on Topcoat's
//! request context, response conventions, and server-sent events:
//!
//! - **Signal reading**: [`Signals`] extracts the signals a Datastar action sends with every
//!   request, and [`datastar_request`] detects those requests from a `cx: &Cx`.
//! - **Events**: [`PatchElements`], [`PatchSignals`], and [`ExecuteScript`] build the events
//!   Datastar consumes. Each converts into a server-sent
//!   [`Event`](topcoat_router::content::sse::Event) for streaming over an
//!   [`Sse`](topcoat_router::content::sse::Sse) response, and returned on its own from a handler it
//!   becomes a single-event stream.
//! - **Responders** ([`DatastarSelector`], [`DatastarMode`], ...) implement
//!   [`IntoResponseParts`](topcoat_router::IntoResponseParts) to set the headers Datastar reads on
//!   plain `text/html`, `application/json`, and `text/javascript` responses.
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
