mod components;

use topcoat::{
    Result,
    mdx::compile_mdx,
    router::{Router, RouterBuilderDiscoverExt, layout, page},
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

// --- Layout -----------------------------------------------------------------

#[layout("/")]
async fn root_layout(slot: Result) -> Result {
    view! {
        <html>
            <head>
                <title>"MDX Docs"</title>
                topcoat::dev::script()
            </head>
            <body>
                <nav>
                    <a href="/footnotes">"Footnotes"</a>
                    " | "
                    <a href="/references">"Reference Links"</a>
                    " | "
                    <a href="/overrides">"Overrides"</a>
                    " | "
                    <a href="/wrappers">"Wrappers"</a>
                    " | "
                    <a href="/code-blocks">"Code Blocks"</a>
                    " | "
                    <a href="/heading-ids">"Heading IDs"</a>
                </nav>
                <hr />
                (slot?)
            </body>
        </html>
    }
}

// --- Pages ------------------------------------------------------------------

#[page("/")]
async fn home() -> Result {
    view! {
        <h1>"MDX Features"</h1>
        <p>"This example demonstrates advanced MDX features in Topcoat."</p>
        <ul>
            <li><a href="/footnotes">"Footnotes"</a></li>
            <li><a href="/references">"Reference Links"</a></li>
            <li><a href="/overrides">"Element Overrides"</a></li>
            <li><a href="/wrappers">"Content Wrappers"</a></li>
            <li><a href="/code-blocks">"Code Block Meta"</a></li>
            <li><a href="/heading-ids">"Heading IDs"</a></li>
        </ul>
    }
}

#[page("/footnotes")]
async fn footnotes() -> Result {
    compile_mdx!("pages/footnotes.mdx")
}

#[page("/references")]
async fn references() -> Result {
    compile_mdx!("pages/references.mdx")
}

#[page("/overrides")]
async fn overrides() -> Result {
    compile_mdx!(
        mdx_components!{},
        overrides = { "a" => components::branded_link },
        "pages/overrides.mdx"
    )
}

#[page("/wrappers")]
async fn wrappers() -> Result {
    compile_mdx!(
        mdx_components! {},
        wrapper = components::page_wrapper,
        "pages/wrappers.mdx"
    )
}

#[page("/code-blocks")]
async fn code_blocks() -> Result {
    compile_mdx!("pages/code-blocks.mdx")
}

#[page("/heading-ids")]
async fn heading_ids() -> Result {
    compile_mdx!("pages/heading-ids.mdx")
}
