//! Session authentication for Topcoat, with storage owned by the application.
//!
//! Use this crate through `topcoat::session`, which also hosts the guide.

#![cfg_attr(docsrs, feature(doc_cfg))]

mod config;
#[cfg(feature = "router")]
mod router;
mod session;
mod state;
mod token;

pub use config::*;
#[cfg(feature = "router")]
pub use router::*;
pub use session::*;
pub use state::*;
pub use token::*;
