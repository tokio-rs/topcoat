mod _group;
mod about;

use topcoat::{
    router::{Slot, layout, page},
    view::{View, view},
};

pub fn router() -> topcoat::router::Router {
    topcoat::router::file_router!()
}

#[layout]
async fn layout(slot: Slot) -> View {
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
    let x = 5+6;

                "current page: "
                (slot.await)

        match 5 {
                6|8 => <div>"hi"</div>,
                7 => {
            <br>
            <span>"bye"</span>
        },
        _ =>
            <span>"rest"</span>

            }


    //before if
                if {
        let kek = 5;
        view! { <!DOCTYPE   html><div>"hi"</div>};


        // big if true
        kek == 6
    } {
        // in if
                "hello"
            } else {
                <div>"bye"</div>
            }

            for kek in [1, 2, 3] {
            <div>"hi"</div>
        }
            </body>
        </html>
    }
}

#[page]
async fn home_page() -> View {
    view! { "home" }
}
