mod docs;
mod posts;

use topcoat::{
    Result,
    router::{Slot, href, layout, module_router, page, query_params},
    view::{View, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(module_router!().build()).await.unwrap();
}

// --- Layout -----------------------------------------------------------------

#[layout]
async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>topcoat::dev::script()</head>
            <body>(slot)</body>
        </html>
    })
}

// --- Home -------------------------------------------------------------------

#[page]
async fn home() -> Result<impl View> {
    Ok(view! {
        <h1>"Path and query params"</h1>
        <ul>
            // `href` builds the URL from the page it points at: `query` adds
            // query items, and the tuple fills the path's parameters.
            <li>
                <a
                    href=(href!(crate::posts::posts).query(
                        [("page", "2"), ("q", "rust")],
                    ))
                >
                    "query params: /posts?page=2&q=rust"
                </a>
            </li>
            <li>
                <a
                    href=(href!(
                        crate::posts::post_id::post,
                        crate::posts::post_id::PostId(42),
                    ))
                >
                    "path param: /posts/42"
                </a>
            </li>
            <li>
                <a
                    href=(href!(
                        crate::docs::doc_path::document,
                        crate::docs::doc_path::DocPath(["guides", "getting-started"]),
                    ))
                >
                    "catch-all param: /docs/guides/getting-started"
                </a>
            </li>
        </ul>
    })
}

// --- Query params -----------------------------------------------------------

// A value that does not parse redirects to the page with the query cleared.
#[query_params(error = redirect("?"))]
struct PostsQuery {
    page: Option<u32>,
    q: Option<String>,
}
