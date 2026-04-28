mod _group;
mod about;

use topcoat::{
    context::{Cx, uri},
    router::{Slot, layout, page},
    view::{View, view},
};

pub fn router() -> topcoat::router::Router {
    topcoat::router::file_router!()
}

#[layout]
async fn layout(cx: &Cx, slot: Slot) -> View {
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
