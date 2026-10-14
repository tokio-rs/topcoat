use std::sync::OnceLock;

use http_body_util::BodyExt;
use serde::{Deserialize, Serialize};
use topcoat::{
    Result,
    context::Cx,
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
async fn fetch(request: HttpRequest, _env: Env, _ctx: Context) -> worker::Result<Response> {
    // The Workers request body is a JavaScript stream, which cannot move
    // between threads, so it is read into memory first.
    let (parts, body) = request.into_parts();
    let body = body.collect().await?.to_bytes();
    Ok(router()
        .handle(Request::from_parts(parts, Body::from(body)))
        .await)
}

// --- Router -----------------------------------------------------------------

// The router is built on the first request and reused by later requests to
// the same Worker instance.
fn router() -> &'static Router {
    static ROUTER: OnceLock<Router> = OnceLock::new();
    ROUTER.get_or_init(|| {
        Router::builder()
            .layout(root_layout)
            .page(home)
            .page(greet)
            .route(submit)
            .build()
    })
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
async fn home() -> Result<impl View> {
    Ok(view! {
        <h1>"Hello from a Worker"</h1>
        <form method="post" action=(href!(submit))>
            <input name="name" placeholder="Your name" required="true">
            <button>"Greet"</button>
        </form>
    })
}

#[derive(Deserialize, Serialize)]
struct Greeting {
    name: String,
}

#[page("/greet")]
async fn greet(Form(input): Form<Greeting>) -> Result<impl View> {
    Ok(view! {
        <h1>
            "Hello, "
            (input.name)
            "!"
        </h1>
        <a href=(href!(home))>"Back"</a>
    })
}

// --- Routes -----------------------------------------------------------------

// Reads the form from the request body, then redirects to the page that
// shows it.
#[route(POST "/greet")]
async fn submit(cx: &Cx, Form(input): Form<Greeting>) -> Result<SeeOther> {
    Ok(see_other(href!(greet).query(input).resolve(cx)))
}
