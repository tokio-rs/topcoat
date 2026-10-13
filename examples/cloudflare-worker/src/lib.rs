use std::sync::OnceLock;

use http_body_util::BodyExt;
use serde::Deserialize;
use toasty::Db;
use toasty_driver_d1::D1;
use topcoat::{
    Result,
    context::{Cx, app_context},
    router::{
        Body, Router, Slot,
        content::Form,
        error::{SeeOther, see_other},
        href, layout, page,
        request::Request,
        response::Response,
        route,
    },
    view::{View, view},
};
use worker::{Context, Env, HttpRequest, event};

// --- Worker -----------------------------------------------------------------

// Cloudflare calls this for every request. There is no server to start: the
// handler passes each request to `Router::handle` and returns its response.
#[event(fetch)]
async fn fetch(request: HttpRequest, env: Env, _ctx: Context) -> worker::Result<Response> {
    // The Workers request body is a JavaScript stream, which cannot move
    // between threads, so it is read into memory first.
    let (parts, body) = request.into_parts();
    let body = body.collect().await?.to_bytes();
    Ok(router(&env)
        .await?
        .handle(Request::from_parts(parts, Body::from(body)))
        .await)
}

// --- Router -----------------------------------------------------------------

// The router and its database handle are built on the first request and
// reused by later requests to the same Worker instance.
async fn router(env: &Env) -> worker::Result<&'static Router> {
    static ROUTER: OnceLock<Router> = OnceLock::new();

    if let Some(router) = ROUTER.get() {
        return Ok(router);
    }

    // `DB` is the D1 binding declared in `wrangler.toml`.
    let db = Db::builder()
        .models(toasty::models!(crate::*))
        .build(D1::new(env.d1("DB")?))
        .await
        .map_err(|error| worker::Error::RustError(error.to_string()))?;

    Ok(ROUTER.get_or_init(|| {
        Router::builder()
            .app_context(db)
            .layout(root_layout)
            .page(home)
            .route(create)
            .build()
    }))
}

// Toasty statements require a mutable database handle.
// Cloning `Db` is inexpensive because it is a handle to the underlying pool.
fn db(cx: &Cx) -> Db {
    app_context::<Db>(cx).clone()
}

// --- Models -----------------------------------------------------------------

// The `todos` table is created by the migration in `migrations/`.
#[derive(Debug, toasty::Model)]
struct Todo {
    #[key]
    #[auto]
    id: u64,

    title: String,
}

// --- Layouts ----------------------------------------------------------------

#[layout("/")]
async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head><title>"Topcoat on Cloudflare Workers"</title></head>
            <body>(slot)</body>
        </html>
    })
}

// --- Pages ------------------------------------------------------------------

#[page("/")]
async fn home(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <h1>"Todos on a Worker"</h1>

        <form method="post" action=(href!(create))>
            <input name="title" placeholder="What needs doing?" required="true">
            <button>"Add"</button>
        </form>

        let todos = Todo::all()
            .order_by(Todo::fields().id().asc())
            .exec(&mut db(cx))
            .await?;

        if todos.is_empty() {
            <p>"All done!"</p>
        } else {
            <ul>
                for todo in todos {
                    <li>(todo.title)</li>
                }
            </ul>
        }
    })
}

// --- Routes -----------------------------------------------------------------

#[derive(Deserialize)]
struct NewTodo {
    title: String,
}

// Stores the todo in D1, then redirects to the list.
#[route(POST "/todos")]
async fn create(cx: &Cx, Form(new_todo): Form<NewTodo>) -> Result<SeeOther> {
    let title = new_todo.title.trim();

    if !title.is_empty() {
        toasty::create!(Todo { title }).exec(&mut db(cx)).await?;
    }

    // Post/Redirect/Get, so a reload does not submit the form again.
    Ok(see_other(href!(home).resolve(cx)))
}
