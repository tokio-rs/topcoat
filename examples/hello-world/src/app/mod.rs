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

#[memoize]
fn add(cx: &Cx, x: &str, y: i32) -> String {
    println!("adding {x} + {y}");
    x.to_owned() + &y.to_string()
}

#[page]
async fn home_page(cx: &Cx) -> View {
    let result1 = add(cx, "5", 6);
    let result1 = add(cx, "5", 6);

    view! { "home" }
}

fn add2<'__cx>(
    cx: &'__cx ::topcoat::context::Cx,
    x: &str,
    y: i32,
) -> ::topcoat::context::Memoized<'__cx, String> {
    cx.cache().memoize((x, y), |(x, y)| {
        {
            ::std::io::_print(format_args!("adding {0} + {1}\n", x, y));
        };
        x.to_owned() + &y.to_string()
    })
}
