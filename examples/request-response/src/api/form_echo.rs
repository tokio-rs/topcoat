use topcoat::{
    Result,
    router::{content::Form, route},
};

use crate::Search;

// For other methods, Form<T> reads and writes application/x-www-form-urlencoded bodies.
#[route(POST)]
pub(crate) async fn form_echo(Form(input): Form<Search>) -> Result<Form<Search>> {
    Ok(Form(input))
}
