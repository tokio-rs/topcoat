use topcoat::{
    Result,
    mdx::mdx_pages,
    router::{Router, RouterBuilderDiscoverExt, layout, page},
    view::{View, component, view},
};

// --- Components --------------------------------------------------------------

#[component]
pub async fn highlight(#[default] child: View) -> Result {
    view! { <span class="bg-yellow-200 px-1 rounded">(child)</span> }
}

// --- Pages -------------------------------------------------------------------

// `mdx_pages!` must be at module level. It scans `pages/` at compile time and
// registers a `PageFn` per file in the link-time inventory, so `/home` and
// `/features` exist without a hand-written handler. The shared `components`
// registry applies to every file in the scan.
mdx_pages!(
    "pages",
    components = {
        Highlight => highlight,
    }
);

// The scan also emits an index, which this handler renders as a listing.
#[page("/")]
async fn index() -> Result {
    let pages = mdx_index_pages();
    view! {
        <h1>"MDX Pages"</h1>
        <ul>
            for entry in pages {
                <li><a href=(entry.path)>(entry.title.unwrap_or(entry.slug))</a></li>
            }
        </ul>
    }
}

// --- Layout ------------------------------------------------------------------

#[layout]
async fn root_layout(slot: Result) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                <title>"MDX Pages"</title>
                topcoat::dev::script()
            </head>
            <body>
                <nav class="border-b px-6 py-3">
                    <a href="/" class="font-semibold">"Index"</a>
                </nav>
                <main class="mx-auto max-w-3xl px-6 py-8">(slot?)</main>
            </body>
        </html>
    }
}

// --- Server ------------------------------------------------------------------

#[tokio::main]
async fn main() {
    // `.discover()` collects the inventory entries submitted by `mdx_pages!`
    // alongside the `#[page]` and `#[layout]` items above.
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}
