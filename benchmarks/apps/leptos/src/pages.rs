//! Page components, one module per page. Pages that show data load it with a server function.

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
