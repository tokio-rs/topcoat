use topcoat::{
    Result,
    context::Cx,
    router::{
        error::{SeeOther, see_other},
        href, path_param, route,
    },
};

use crate::{Todo, db, home};

#[route(POST)]
pub(crate) async fn toggle(cx: &Cx) -> Result<SeeOther> {
    let mut db = db(cx);

    let mut todo =
        Todo::get_by_id(&mut db, *path_param::<crate::todos::todo_id::TodoId>(cx)?).await?;
    let done = !todo.done;

    toasty::update!(todo { done }).exec(&mut db).await?;

    Ok(see_other(href!(home).resolve(cx)))
}
