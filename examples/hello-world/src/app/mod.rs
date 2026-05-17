mod _group;
mod api;
mod posts;

use std::time::Duration;

use axum::response::Html;
use topcoat::{
    asset::asset,
    context::{Cx, memoize},
    router::{IntoResponse, Response, Result, Slot, layout, page, query_params, route},
    tailwind,
    view::view,
};

use crate::components::app_and_request_state;

pub fn router() -> topcoat::router::Router {
    topcoat::router::module_router!()
}

// Pretend this is an expensive database lookup. The `println!` makes it obvious in the
// terminal that the body only runs once per request, even though both the layout and the
// page call it.
#[memoize]
async fn current_user(cx: &Cx) -> String {
    println!("loading current user");
    "alice".to_owned()
}

#[layout]
async fn layout(cx: &Cx, slot: Slot<'_>) -> Result {
    let user = current_user(cx).await;
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"hello world"</title>
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
                <script type="module" src=(asset!("https://cdn.jsdelivr.net/gh/starfederation/datastar@v1.0.1/bundles/datastar.js"))></script>
                [topcoat::dev::script /]
            </head>
            <body>
                <nav>
                    <a href="/">"home"</a>
                    <span>" | "</span>
                    <a href="/about">"about"</a>
                    <span class=("test")>" | "</span>
                    <a href="/contact">"contact"</a>
                    <span-kek:pip
                        for kek in ["kek", "pip", "lel"] {
                            if kek != "pip" {
                                (kek)=(kek)
                            }
                        }
                    >
                        " | signed in as "
                        ((*user).clone())
                    </span-kek:pip>
                </nav>
                <hr>

                [app_and_request_state /]

                <div>(slot.await?)</div>
            </body>
        </html>
    }
}

mod about {
    use topcoat::{
        asset::asset,
        router::{Result, page},
        view::view,
    };

    #[page]
    async fn about_page() -> Result {
        view! {
            <div class="font-bold">"about"</div>
            <img
                src=(asset!(
                    "https://upload.wikimedia.org/wikipedia/commons/thumb/d/d5/Rust_programming_language_black_logo.svg/960px-Rust_programming_language_black_logo.svg.png?utm_source=commons.wikimedia.org&utm_campaign=index&utm_content=thumbnail",
                    rename : "rust"
                ))
            >
            <img src=(asset!("./ferris.png"))>
        }
    }
}

#[query_params]
struct DatastarQueryParams {
    datastar: String,
}

#[derive(serde::Deserialize)]
struct Signals {
    input: String,
}

#[route(GET "/content")]
async fn content(cx: &Cx) -> Result<Response> {
    let kek = DatastarQueryParams::of(cx)
        .as_ref()
        .ok()
        .and_then(|s| serde_json::from_str(&s.datastar).ok())
        .unwrap_or(Signals {
            input: "".to_owned(),
        });

    tokio::time::sleep(Duration::from_secs_f32(0.5)).await;
    println!("rendering!");

    let result: Result = view! {
        <div id="content">
            "lol"
            (kek.input)
        </div>
    };
    Ok(Html(result?.render(cx)).into_response())
}

#[page]
async fn home_page(cx: &Cx) -> Result {
    view! {
        <input class="border border-[black]" data-bind="test">
        <div data-effect="[true, $test] && @get('/content', { payload: { input: $test }})">
            <div id="content">"loading..."</div>
        </div>
    }
}
