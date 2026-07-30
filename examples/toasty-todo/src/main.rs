use serde::Deserialize;
use toasty::Db;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Router, RouterBuilderDiscoverExt,
        content::Form,
        error::{SeeOther, see_other},
        layout, page, path_param, route,
    },
    view::{component, view},
};

#[tokio::main]
async fn main() {
    // Use an in-memory SQLite database to keep the example self-contained.
    // Replace this with `sqlite:todos.db` to persist todos across restarts.
    let db = Db::builder()
        .models(toasty::models!(crate::*))
        .connect("sqlite::memory:")
        .await
        .unwrap();

    // Create the database schema generated from the Toasty models.
    db.push_schema().await.unwrap();

    // Register the database as application context, discover the routes,
    // and start the server at http://127.0.0.1:3000 by default.
    topcoat::start(Router::builder().discover().app_context(db).build())
        .await
        .unwrap();
}

// Toasty statements require a mutable database handle.
// Cloning `Db` is inexpensive because it is a handle to the underlying pool.
fn db(cx: &Cx) -> Db {
    app_context::<Db>(cx).clone()
}

// This model defines the structure of a todo stored in SQLite.
#[derive(Debug, toasty::Model)]
struct Todo {
    #[key]
    #[auto]
    id: u64,

    title: String,

    done: bool,
}

#[layout("/")]
async fn root(slot: Result) -> Result {
    // Wrap every page in a complete HTML document.
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Toasty Todos"</title>
                topcoat::dev::script()
            </head>
            <body>(slot?)</body>
        </html>
    }
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    view! {
        <h1>"Toasty Todos"</h1>

        // Submit a new todo to POST /todos.
        <form method="post" action="/todos">
            <input type="text" name="title" placeholder="What needs doing?" required="">
            <button type="submit">"Add"</button>
        </form>

        // Load all todos and display them in creation order.
        let todos = Todo::all()
            .order_by(Todo::fields().id().asc())
            .exec(&mut db(cx))
            .await?;

        if todos.is_empty() {
            <p>"All done!"</p>
        } else {
            <ul
                style="list-style: none; padding: 0; display: flex; \
                    flex-direction: column; gap: 0.375em;"
            >
                for todo in todos {
                    <li style="display: flex; align-items: center; gap: 0.5em;">
                        toggle_checkbox(todo: &todo)

                        if todo.done {
                            <s>(&todo.title)</s>
                        } else {
                            (&todo.title)
                        }

                        delete_button(todo: &todo)
                    </li>
                }
            </ul>
        }
    }
}

// --- Components -------------------------------------------------------------

#[component]
async fn toggle_checkbox(todo: &Todo) -> Result {
    // Submit the todo ID when the checkbox changes.
    view! {
        <form method="post" action=(("/todos/", todo.id, "/toggle"))>
            <input type="checkbox" checked=(todo.done) onchange="this.form.submit()">
        </form>
    }
}

#[component]
async fn delete_button(todo: &Todo) -> Result {
    // Submit the todo ID to its delete endpoint.
    view! {
        <form method="post" action=(("/todos/", todo.id, "/delete"))>
            <button type="submit">"delete"</button>
        </form>
    }
}

// --- Routes -----------------------------------------------------------------

#[derive(Deserialize)]
struct NewTodo {
    title: String,
}

#[route(POST "/todos")]
async fn create(cx: &Cx, Form(new_todo): Form<NewTodo>) -> Result<SeeOther> {
    // Ignore titles that contain only whitespace.
    let title = new_todo.title.trim();

    if !title.is_empty() {
        toasty::create!(Todo { title, done: false })
            .exec(&mut db(cx))
            .await?;
    }

    // Use Post/Redirect/Get to return to the todo list.
    Ok(see_other("/"))
}

// Parse the dynamic todo ID as an unsigned integer.
path_param!(todo_id: u64, error = bad_request);

#[route(POST "/todos/{todo_id}/toggle")]
async fn toggle(cx: &Cx) -> Result<SeeOther> {
    let mut db = db(cx);

    // Load the selected todo and invert its completed state.
    let mut todo = Todo::get_by_id(&mut db, *path_param::<TodoId>(cx)?).await?;
    let done = !todo.done;

    toasty::update!(todo { done }).exec(&mut db).await?;

    Ok(see_other("/"))
}

#[route(POST "/todos/{todo_id}/delete")]
async fn delete(cx: &Cx) -> Result<SeeOther> {
    // Delete the selected todo by its ID.
    Todo::delete_by_id(&mut db(cx), *path_param::<TodoId>(cx)?).await?;

    Ok(see_other("/"))
}
