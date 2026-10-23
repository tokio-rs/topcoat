use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{content::Form, error::SeeOther, route},
};

use super::_common::see_back;
use crate::features::theme::toggle_theme;

#[derive(Deserialize)]
pub struct Toggle {
    return_to: String,
}

#[route(POST)]
pub async fn toggle(cx: &Cx, Form(form): Form<Toggle>) -> Result<SeeOther> {
    toggle_theme(cx)?;
    Ok(see_back(cx, &form.return_to))
}
