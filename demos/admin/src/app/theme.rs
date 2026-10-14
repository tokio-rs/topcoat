use serde::Deserialize;
use topcoat::{
    Result,
    context::Cx,
    router::{
        content::Form,
        error::{SeeOther, see_other},
        href, route,
    },
};

use crate::features::theme::toggle_theme;

#[derive(Deserialize)]
pub struct Toggle {
    return_to: String,
}

#[route(POST)]
pub async fn toggle(cx: &Cx, Form(form): Form<Toggle>) -> Result<SeeOther> {
    toggle_theme(cx)?;

    // Only redirect to local paths because the submitted URL is untrusted.
    let local = form.return_to.starts_with('/')
        && !form.return_to.starts_with("//")
        && !form.return_to.contains('\\');
    if local {
        Ok(see_other(form.return_to))
    } else {
        Ok(see_other(
            href!(super::_admin::_dashboard::page).resolve(cx),
        ))
    }
}
