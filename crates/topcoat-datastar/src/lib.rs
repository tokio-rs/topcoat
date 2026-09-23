//! [Datastar](https://data-star.dev) support for Topcoat.
//!
//! Extract request signals with [`Signals`] and return updates such as
//! [`PatchElements`] from handlers. Updates can also be converted into an
//! [`Event`](topcoat_router::content::sse::Event) and sent over an
//! [`Sse`](topcoat_router::content::sse::Sse) stream.
//!
//! Response header types implement
//! [`IntoResponseParts`](topcoat_router::response::IntoResponseParts) to control
//! ordinary responses. Raw header names are available in [`header`].

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
