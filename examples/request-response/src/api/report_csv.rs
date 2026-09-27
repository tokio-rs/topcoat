use topcoat::router::segment;

use crate::Csv;
use topcoat::{Result, router::route};

// Returning a custom IntoResponse type lets the handler choose headers and body.
segment!(rename = "report.csv");

#[route(GET)]
pub(crate) async fn report() -> Result<Csv> {
    Ok(Csv("name,total\nAda,42\nGrace,64\n".to_string()))
}
