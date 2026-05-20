mod _group;
mod api;
mod posts;

use std::time::Duration;

use topcoat::{
    asset::asset,
    context::{Cx, memoize},
    router::{IntoResponse, Response, Result, Slot, layout, page, query_params, route},
    tailwind,
    view::{ReadSignal, Signal, component, island, view},
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
                <script type="module" src=(asset!("./runtime.js"))></script>
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
#[page]
async fn home_page() -> Result {
    view! {
        [combobox /]
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

async fn search_results(cx: &Cx, input: &str) -> Vec<&'static str> {
    tokio::time::sleep(Duration::from_secs_f32(0.5)).await;
    let all = [
        "apple",
        "apricot",
        "banana",
        "blackberry",
        "blueberry",
        "cherry",
        "coconut",
        "cranberry",
        "date",
        "dragonfruit",
        "elderberry",
        "fig",
        "grape",
        "grapefruit",
        "guava",
        "honeydew",
        "kiwi",
        "lemon",
        "lime",
        "lychee",
        "mango",
        "nectarine",
        "orange",
        "papaya",
        "passionfruit",
        "peach",
        "pear",
        "persimmon",
        "pineapple",
        "plum",
        "pomegranate",
        "raspberry",
        "strawberry",
        "tangerine",
        "watermelon",
    ];
    let needle = input.to_lowercase();
    all.into_iter().filter(|s| s.contains(&needle)).collect()
}

#[island]
async fn combobox_content(cx: &Cx, input: ReadSignal<String>) -> Result {
    let results = search_results(cx, &*input).await;
    view! {
        <div>
            <b>"resuls:"</b>
            for item in results {
                <div>(item)</div>
            }
        </div>
    }
}

#[component]
async fn combobox(cx: &Cx) -> Result {
    view! {
        signal kek = "initial content".to_owned();

        <div>
            <input data-topcoat-bind=(kek.clone())>
            track combobox_content(kek)
        </div>
    }
}
