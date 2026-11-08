use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        href, route,
    },
};

use crate::{Todo, db, home};

#[route(POST)]
pub(crate) async fn delete(cx: &Cx) -> Result<SeeOther> {
    Todo::delete_by_id(
        &mut db(cx),
        *crate::todos::todo_id::todo_id(cx)?,
    )
    .await?;

    Ok(see_other(href!(home).resolve(cx)))
}
