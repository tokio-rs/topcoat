mod _group;
mod about;

use topcoat::{
    context::{Cx, uri},
    memoize,
    router::{Slot, layout, page},
    view::{View, view},
};

pub fn router() -> topcoat::router::Router {
    topcoat::router::file_router!()
}

#[memoize]
async fn kek(cx: &Cx, x: i32, y: i32) -> i32 {
    println!("adding {x} + {y}");
    x + y
}

#[memoize]
async fn pip(cx: &Cx, x: i32, y: i32) -> i32 {
    println!("adding {x} + {y} in pip");
    x + y
}

#[layout]
async fn layout(cx: &Cx, slot: Slot) -> View {
    let result = kek(cx, 5, 6).await;
    let result = kek(cx, 5, 6).await;
    let result = pip(cx, 5, 6).await;
    let result = pip(cx, 5, 6).await;
    let result = pip(cx, 5, 6).await;

    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"hello world"</title>
                [topcoat::dev::script /]
            </head>
            <body>
                <nav>
                    <a href="/">"home"</a>
                    <span>" | "</span>
                    <a href="/about">"about"</a>
                    <span class=("test")>" | "</span>
                    <a href="/contact">"contact"</a>
                </nav>
                <hr>

                "current page: "
                (uri(cx).to_string())

                <div>
                    (slot.await)
                </div>
            </body>
        </html>
    }
}

#[page]
async fn home_page(cx: &Cx) -> View {
    view! { "home" }
}
