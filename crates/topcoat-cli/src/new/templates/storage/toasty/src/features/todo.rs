//! The todo list, stored in the database.

use topcoat::{Result, context::Cx};

use crate::db::db;

/// A todo item.
#[derive(Debug, toasty::Model)]
pub struct Todo {
    #[key]
    #[auto]
    pub id: u64,

    pub title: String,

    pub done: bool,
}

impl Todo {
    /// Returns every todo, oldest first.
    pub async fn list(cx: &Cx) -> Result<Vec<Todo>> {
        let todos = Todo::all()
            .order_by(Todo::fields().id().asc())
            .exec(&mut db(cx))
            .await?;
        Ok(todos)
    }

    /// Adds a todo that is not done yet.
    pub async fn add(cx: &Cx, title: &str) -> Result<()> {
        toasty::create!(Todo { title, done: false })
            .exec(&mut db(cx))
            .await?;
        Ok(())
    }

    /// Marks a todo as done, or as not done if it already is.
    pub async fn toggle(cx: &Cx, id: u64) -> Result<()> {
        let mut db = db(cx);
        let mut todo = Todo::get_by_id(&mut db, &id).await?;
        let done = !todo.done;
        todo.update().done(done).exec(&mut db).await?;
        Ok(())
    }

    /// Removes a todo.
    pub async fn remove(cx: &Cx, id: u64) -> Result<()> {
        Todo::filter_by_id(id).delete().exec(&mut db(cx)).await?;
        Ok(())
    }
}
