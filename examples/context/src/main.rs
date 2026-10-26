use std::sync::atomic::{AtomicU64, Ordering};

use topcoat::{
    Result,
    context::{Cx, context},
    router::{href, module_router, page, request::uri},
    view::{View, component, view},
};

// Requests use the same counter stored on the router.
struct PageViews(AtomicU64);

// The router supplies the greeting unless the layer in `formal` overrides it.
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

    // Give handlers under /formal a child context with a different greeting.
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

// Plain helpers can use Cx to read request data too.
fn current_path(cx: &Cx) -> &str {
    uri(cx).path()
}

// Both pages use this component. It gets its greeting from the context.
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
