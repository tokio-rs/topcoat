use crate::{Todo, db, home};
use topcoat::{Result, context::Cx, router::{error::{SeeOther, see_other}, href, path_param, route}};

#[route(POST)]
pub(crate) async fn delete(cx: &Cx) -> Result<SeeOther> {
    Todo::delete_by_id(
        &mut db(cx),
        *path_param::<crate::todos::todo_id::TodoId>(cx)?,
    )
    .await?;

    Ok(see_other(href!(home).resolve(cx)))
}
