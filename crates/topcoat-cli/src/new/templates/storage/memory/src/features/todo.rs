//! The todo list, kept in memory until the server stops.

use std::sync::{Mutex, MutexGuard};

use topcoat::{
    Result,
    context::{Cx, app_context},
};

/// A todo item.
#[derive(Clone, Debug)]
pub struct Todo {
    pub id: u64,
    pub title: String,
    pub done: bool,
}

impl Todo {
    /// Returns every todo, oldest first.
    pub async fn list(cx: &Cx) -> Result<Vec<Todo>> {
        Ok(todos(cx).items.clone())
    }

    /// Adds a todo that is not done yet.
    pub async fn add(cx: &Cx, title: &str) -> Result<()> {
        let mut todos = todos(cx);
        todos.next_id += 1;
        let id = todos.next_id;
        todos.items.push(Todo {
            id,
            title: title.to_string(),
            done: false,
        });
        Ok(())
    }

    /// Marks a todo as done, or as not done if it already is.
    pub async fn toggle(cx: &Cx, id: u64) -> Result<()> {
        if let Some(todo) = todos(cx).items.iter_mut().find(|todo| todo.id == id) {
            todo.done = !todo.done;
        }
        Ok(())
    }

    /// Removes a todo.
    pub async fn remove(cx: &Cx, id: u64) -> Result<()> {
        todos(cx).items.retain(|todo| todo.id != id);
        Ok(())
    }
}

/// The todo items, shared through the router's app context.
#[derive(Default)]
pub struct Todos(Mutex<TodoList>);

#[derive(Default)]
struct TodoList {
    items: Vec<Todo>,
    next_id: u64,
}

fn todos(cx: &Cx) -> MutexGuard<'_, TodoList> {
    app_context::<Todos>(cx).0.lock().unwrap()
}
