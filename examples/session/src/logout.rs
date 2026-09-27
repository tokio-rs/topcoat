use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        href, route,
    },
};

use crate::{db, session};

#[route(POST)]
pub(crate) async fn logout(cx: &Cx) -> Result<SeeOther> {
    if let Some(token_hash) = session::stop(cx).await? {
        db(cx).delete(&token_hash);
    }

    Ok(see_other(href!(crate::page).resolve(cx)))
}
