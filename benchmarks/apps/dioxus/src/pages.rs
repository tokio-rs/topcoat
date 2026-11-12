//! The routed pages, one module each, with the server functions that load their data.

mod about;
mod detail;
mod error;
mod home;
mod listing;
mod orders;

pub use about::*;
pub use detail::*;
pub use error::*;
pub use home::*;
pub use listing::*;
pub use orders::*;
