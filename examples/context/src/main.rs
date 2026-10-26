use std::sync::atomic::{AtomicU64, Ordering};

use topcoat::{
    Result,
    context::{Cx, context},
    router::{href, module_router, page, request::uri},
    view::{View, component, view},
};

// Registered on the router, so every request shares this counter.
struct PageViews(AtomicU64);

// The router registers a default greeting. The layer in `formal` replaces it
// for every page under /formal.
struct Greeting(&'static str);

#[tokio::main]
async fn main() {
    topcoat::start(
        module_router!()
            .with(PageViews(AtomicU64::new(0)))
            .with(Greeting("Hi"))
            .build(),
    )
    .await
    .unwrap();
}

#[page]
async fn home() -> Result<impl View> {
    Ok(view! { greeting_page() })
}

mod formal {
    use topcoat::{
        Result,
        context::Cx,
        router::{Body, Next, layer, page, response::Response},
        view::{View, view},
    };

    use super::{Greeting, greeting_page};

    // Runs before every page under /formal. Handlers below it see the
    // greeting it adds instead of the router's.
    #[layer]
    async fn layer(cx: &Cx, body: Body, next: Next<'_>) -> Result<Response> {
        let cx = cx.with(Greeting("Good day"));
        next.run(&cx, body).await
    }

    #[page]
    pub async fn page() -> Result<impl View> {
        Ok(view! { greeting_page() })
    }
}

// A plain function can read the request through `Cx`.
fn current_path(cx: &Cx) -> &str {
    uri(cx).path()
}

// Both pages render this component. It reads whichever greeting is in its
// context, so it needs no arguments.
#[component]
async fn greeting_page(cx: &Cx) -> Result<impl View> {
    let views = context::<PageViews>(cx).0.fetch_add(1, Ordering::Relaxed) + 1;
    let greeting = context::<Greeting>(cx);

    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"Context"</title>
                topcoat::dev::script()
            </head>
            <body>
                <h1>(greeting.0)</h1>

                <p>
                    "path: "
                    (current_path(cx))
                </p>

                <p>
                    "Pages have been viewed "
                    (views)
                    " times."
                </p>

                <nav>
                    <a href=(href!(home))>"Casual"</a>
                    " "
                    <a href=(href!(formal::page))>"Formal"</a>
                </nav>
            </body>
        </html>
    })
}
