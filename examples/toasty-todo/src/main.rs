use serde::Deserialize;
use toasty::Db;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Router, RouterBuilderDiscoverExt, Slot,
        content::Form,
        error::{SeeOther, see_other},
        href, layout, page, path_param, route,
    },
    view::{View, component, view},
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

    db.push_schema().await.unwrap();

    topcoat::start(Router::builder().discover().app_context(db).build())
        .await
        .unwrap();
}

// Toasty statements require a mutable database handle.
// Cloning `Db` is inexpensive because it is a handle to the underlying pool.
fn db(cx: &Cx) -> Db {
    app_context::<Db>(cx).clone()
}

#[derive(Debug, toasty::Model)]
struct Todo {
    #[key]
    #[auto]
    id: u64,

    title: String,

    done: bool,
}

#[layout("/")]
async fn root(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Toasty Todos"</title>
                topcoat::dev::script()
            </head>
            <body>(slot)</body>
        </html>
    })
}

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <h1>"Toasty Todos"</h1>

        <form method="post" action=(href!(create))>
            <input type="text" name="title" placeholder="What needs doing?" required="">
            <button type="submit">"Add"</button>
        </form>

        // A `view!` body can run statements, including awaiting a query.
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
    })
}

// --- Components -------------------------------------------------------------

#[component]
async fn toggle_checkbox(todo: &Todo) -> Result<impl View> {
    Ok(view! {
        <form method="post" action=(href!(toggle, TodoId(todo.id)))>
            <input type="checkbox" checked=(todo.done) onchange="this.form.submit()">
        </form>
    })
}

#[component]
async fn delete_button(todo: &Todo) -> Result<impl View> {
    Ok(view! {
        <form method="post" action=(href!(delete, TodoId(todo.id)))>
            <button type="submit">"delete"</button>
        </form>
    })
}

// --- Routes -----------------------------------------------------------------

#[derive(Deserialize)]
struct NewTodo {
    title: String,
}

#[route(POST "/todos")]
async fn create(cx: &Cx, Form(new_todo): Form<NewTodo>) -> Result<SeeOther> {
    let title = new_todo.title.trim();

    if !title.is_empty() {
        toasty::create!(Todo { title, done: false })
            .exec(&mut db(cx))
            .await?;
    }

    // Post/Redirect/Get, so a reload does not submit the form again.
    Ok(see_other(href!(home).resolve(cx)))
}

path_param!(todo_id: u64, error = bad_request);

#[route(POST "/todos/{todo_id}/toggle")]
async fn toggle(cx: &Cx) -> Result<SeeOther> {
    let mut db = db(cx);

    let mut todo = Todo::get_by_id(&mut db, *path_param::<TodoId>(cx)?).await?;
    let done = !todo.done;

    toasty::update!(todo { done }).exec(&mut db).await?;

    Ok(see_other(href!(home).resolve(cx)))
}

#[route(POST "/todos/{todo_id}/delete")]
async fn delete(cx: &Cx) -> Result<SeeOther> {
    Todo::delete_by_id(&mut db(cx), *path_param::<TodoId>(cx)?).await?;

    Ok(see_other(href!(home).resolve(cx)))
}
