use topcoat::{Result, mdx::mdx_pages, router::page, view::view};

// `mdx_pages!` must be at module level: it generates consts, functions, and
// inventory registrations that cannot appear inside a function body.
//
// Each discovered page registers as a `PageFn` in the link-time inventory.
// When using `module_router!`, call `.discover()` on the builder to pick
// these up (see `app.rs`).
mdx_pages!("posts", prefix = "/blog");

#[page]
async fn blog() -> Result {
    let posts = mdx_index_posts();
    view! {
        <h1>"Blog Posts"</h1>
        <ul>
            for post in posts {
                <li>
                    <a href=(post.path)>(post.title.unwrap_or(post.slug))</a>
                    " - "
                    (post.date.unwrap_or("undated"))
                </li>
            }
        </ul>
    }
}
