pub(crate) mod bytes;
pub(crate) mod files;
pub(crate) mod form_echo;
pub(crate) mod maybe_user;
pub(crate) mod raw_form;
pub(crate) mod search;
pub(crate) mod signed;
pub(crate) mod upload;
pub(crate) mod users;

use topcoat::{
    Result,
    router::route,
};

use crate::Csv;

// Returning a custom IntoResponse type lets the handler choose headers and body.
#[route(GET "./report.csv")]
pub(crate) async fn report() -> Result<Csv> {
    Ok(Csv("name,total\nAda,42\nGrace,64\n".to_string()))
}
