mod components;

use topcoat::{
    Result,
    mdx::compile_mdx,
    router::{Router, RouterBuilderDiscoverExt, page},
    view::view,
};

// --- Server -----------------------------------------------------------------

#[tokio::main]
async fn main() {
    topcoat::start(router()).await.unwrap();
}

// --- Router -----------------------------------------------------------------

fn router() -> Router {
    Router::builder().discover().build()
}

// --- Pages ------------------------------------------------------------------

#[page("/")]
async fn home() -> Result {
    view! {
        <html>
            <head>
                <title>"MDX Components"</title>
                topcoat::dev::script()
            </head>
            <body>
                <h1>"MDX Components"</h1>
                <p>
                    "This example demonstrates how to use custom components in MDX pages."
                </p>
                <ul>
                    <li><a href="/callouts">"Callouts"</a></li>
                    <li><a href="/wrappers">"Wrappers"</a></li>
                    <li><a href="/self-closing">"Self-closing"</a></li>
                    <li><a href="/nested">"Nested"</a></li>
                </ul>
            </body>
        </html>
    }
}

#[page("/callouts")]
async fn callouts() -> Result {
    compile_mdx!(
        mdx_components! {
            Callout => components::callout,
            Wrapper => components::wrapper,
            Divider => components::divider,
        },
        "pages/callouts.mdx"
    )
}

#[page("/wrappers")]
async fn wrappers() -> Result {
    compile_mdx!(
        mdx_components! {
            Callout => components::callout,
            Wrapper => components::wrapper,
            Divider => components::divider,
        },
        "pages/wrappers.mdx"
    )
}

#[page("/self-closing")]
async fn self_closing() -> Result {
    compile_mdx!(
        mdx_components! {
            Callout => components::callout,
            Wrapper => components::wrapper,
            Divider => components::divider,
        },
        "pages/self-closing.mdx"
    )
}

#[page("/nested")]
async fn nested() -> Result {
    compile_mdx!(
        mdx_components! {
            Callout => components::callout,
            Wrapper => components::wrapper,
            Divider => components::divider,
        },
        "pages/nested.mdx"
    )
}
