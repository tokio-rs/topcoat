//! The catalog and order types shared by the server and the client.
//!
//! The server loads them from the dataset and sends them to the client as part of the server
//! function results that hydrate each page.

mod category;
mod order;
mod product;
mod sort;

pub use category::*;
pub use order::*;
pub use product::*;
pub use sort::*;

// Serializes a `time::Date` as an ISO 8601 date like `2026-03-04`.
time::serde::format_description!(pub iso_date, Date, "[year]-[month]-[day]");
