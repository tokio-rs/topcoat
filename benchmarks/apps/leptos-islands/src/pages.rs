//! Page components, one module per page. Pages read their data from the store in context.

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
