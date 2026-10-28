use std::sync::Arc;

use topcoat::{
    Result,
    context::{ContextMap, Cx, context},
    view::{Child, View, ViewExt, component, view},
};

struct Site {
    title: &'static str,
}

struct Reader {
    name: &'static str,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    // Put values in a shared map when rendering without a router.
    let mut shared = ContextMap::new();
    shared.insert(Site {
        title: "Routerless Topcoat",
    });
    let shared = Arc::new(shared);

    for name in ["Ada & Grace", "World"] {
        // Start a context for this render that reads the shared map.
        let cx = Cx::new()
            .with_shared(Arc::clone(&shared))
            .with(Reader { name });
        let html = render(&cx).await?;
        println!("{html}");
    }

    Ok(())
}

async fn render(cx: &Cx) -> Result<String> {
    // Outside a component, pass the context explicitly to `view!`.
    let view = view! {
        cx =>
        document(
            greeting()
            <p>"Rendered to a string with only the view feature enabled."</p>
        )
    };

    // Resolve the non-live view, then render its content to an HTML string.
    Ok(view.single().await?.render(cx))
}

#[component]
async fn document(cx: &Cx, child: Child<'_>) -> Result<impl View> {
    let site = context::<Site>(cx);

    Ok(view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <title>(site.title)</title>
            </head>
            <body>
                <h1>(site.title)</h1>
                <main>(child)</main>
            </body>
        </html>
    })
}

#[component]
async fn greeting(cx: &Cx) -> Result<impl View> {
    // Nested components receive the context automatically.
    let reader = context::<Reader>(cx);

    Ok(view! {
        <p>
            "Hello, "
            (reader.name)
            "!"
        </p>
    })
}
