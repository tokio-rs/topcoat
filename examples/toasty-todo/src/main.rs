use serde::Deserialize;
use toasty::Db;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Form, HeaderName, Router, RouterBuilderDiscoverExt, RouterErrorExt, Slot, StatusCode,
        header, layout, page, path_param, route,
    },
    view::view,
};

#[tokio::main]
async fn main() {
    // An in-memory database keeps the example self-contained; point the URL at
    // a file (e.g. "sqlite:todos.db") to persist todos across restarts.
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

// Toasty statements borrow the handle mutably, so each handler clones the
// shared `Db` (a cheap handle to the underlying connection pool) out of app
// context.
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
async fn root(slot: Slot<'_>) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Toasty Todos"</title>
                topcoat::dev::script()
            </head>
            <body>(slot.await?)</body>
        </html>
    }
}

#[page("/")]
async fn home(cx: &Cx) -> Result {
    let mut db = db(cx);
    let todos = Todo::all()
        .order_by(Todo::fields().id().asc())
        .exec(&mut db)
        .await?;

    view! {
        <h1>"Toasty Todos"</h1>
        <form method="post" action="/todos">
            <input
                type="text"
                name="title"
                placeholder="What needs doing?"
                required=(true)
            >
            <button type="submit">"Add"</button>
        </form>
        if todos.is_empty() {
            <p>"All done!"</p>
        } else {
            <ul
                style="list-style: none; padding: 0; display: flex; flex-direction: column; gap: 0.375em;"
            >
                for todo in todos {
                    <li style="display: flex; align-items: center; gap: 0.5em;">
                        <form
                            method="post"
                            action=(format!("/todos/{}/toggle", todo.id))
                        >
                            <input
                                type="checkbox"
                                checked=(todo.done)
                                onchange="this.form.submit()"
                            >
                        </form>
                        if todo.done {
                            <s>(todo.title)</s>
                        } else {
                            (todo.title)
                        }
                        <form
                            method="post"
                            action=(format!("/todos/{}/delete", todo.id))
                        >
                            <button type="submit">"delete"</button>
                        </form>
                    </li>
                }
            </ul>
        }
    }
}

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

    Ok(see_other("/"))
}

#[path_param]
struct TodoId(u64);

#[route(POST "/todos/{todo_id}/toggle")]
async fn toggle(cx: &Cx) -> Result<SeeOther> {
    let todo_id = path_param::<TodoId>(cx).ok_or_bad_request("invalid todo id")?;

    let mut db = db(cx);
    let mut todo = Todo::get_by_id(&mut db, **todo_id).await?;
    let done = !todo.done;
    toasty::update!(todo { done }).exec(&mut db).await?;

    Ok(see_other("/"))
}

#[route(POST "/todos/{todo_id}/delete")]
async fn delete(cx: &Cx) -> Result<SeeOther> {
    let todo_id = path_param::<TodoId>(cx).ok_or_bad_request("invalid todo id")?;

    Todo::delete_by_id(&mut db(cx), **todo_id).await?;

    Ok(see_other("/"))
}

// Every mutation above responds with "303 See Other", sending the browser
// back to the todo list with a GET request.
type SeeOther = (StatusCode, [(HeaderName, &'static str); 1]);

fn see_other(uri: &'static str) -> SeeOther {
    (StatusCode::SEE_OTHER, [(header::LOCATION, uri)])
}
