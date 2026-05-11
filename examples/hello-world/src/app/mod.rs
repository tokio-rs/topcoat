mod _group;
mod api;
mod posts;

use topcoat::{
    context::{Cx, memoize},
    router::{Result, Slot, layout, page},
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
                <link rel="stylesheet" href=tailwind::stylesheet!()>
                [topcoat::dev::script /]
            </head>
            <body>
                <nav>
                    <a href="/">"home"</a>
                    <span>" | "</span>
                    <a href="/about">"about"</a>
                    <span class=("test")>" | "</span>
                    <a href="/contact">"contact"</a>
                    <span>
                        " | signed in as "
                        ((*user).clone())
                    </span>
                </nav>
                <hr>

                [app_and_request_state /]

                <div>(slot.await?)</div>
            </body>
        </html>
    }
}

#[page]
async fn home_page(cx: &Cx) -> Result {
    let user = current_user(cx).await;
    view! { "welcome, " ((*user).clone()) }
}

mod about {
    use topcoat::{
        asset::asset,
        router::{Result, page},
        view::view,
    };

    #[page]
    async fn about_page() -> Result {
        view! { <div class="font-bold">"about"</div> <img src=asset!("./ferris.png")> }
    }
}
