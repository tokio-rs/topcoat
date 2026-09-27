pub(crate) mod todo_id;

use crate::{NewTodo, Todo, db, home};
use topcoat::{Result, context::Cx, router::{content::Form, error::{SeeOther, see_other}, href, route}};

#[route(POST)]
pub(crate) async fn create(cx: &Cx, Form(new_todo): Form<NewTodo>) -> Result<SeeOther> {
    let title = new_todo.title.trim();

    if !title.is_empty() {
        toasty::create!(Todo { title, done: false })
            .exec(&mut db(cx))
            .await?;
    }

    // Post/Redirect/Get, so a reload does not submit the form again.
    Ok(see_other(href!(home).resolve(cx)))
}
