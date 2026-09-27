use crate::{LoginForm, User, db, session};
use topcoat::{Result, context::Cx, router::{content::Form, error::{SeeOther, see_other}, href, page, route}};

#[route(POST)]
pub(crate) async fn login(cx: &Cx, Form(form): Form<LoginForm>) -> Result<SeeOther> {
    // A real application would verify credentials before starting the session.
    let session = session::start(cx).await?;

    db(cx).create(session, User { name: form.name });

    Ok(see_other(href!(page).resolve(cx)))
}
