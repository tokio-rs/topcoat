mod admin;
mod posts;
mod rewrite;
mod rewritten;

use topcoat::{Result, router::{Slot, StatusCode, error::{ForbiddenError, NotFoundError, rewrite}, href, layout, module_router, not_found, page}, view::{View, error_boundary, view}};

#[tokio::main]
async fn main() {
    topcoat::start(module_router!().build()).await.unwrap();
}

#[page]
async fn home() -> Result<impl View> {
    Ok(view! {
        <h1>"Error handling"</h1>
        <ul>
            <li>
                <a
                    href=(href!(
                        crate::posts::post_id::post,
                        crate::posts::post_id::PostId(1),
                    ))
                >
                    "An existing post"
                </a>
            </li>
            <li>
                <a
                    href=(href!(
                        crate::posts::post_id::post,
                        crate::posts::post_id::PostId(7),
                    ))
                >
                    "A missing post (404)"
                </a>
            </li>
            <li><a href=(href!(crate::admin::admin))>"The admin area (403)"</a></li>
            <li><a href=(href!(crate::rewrite::rewrite_page))>"A page rewrite"</a></li>
            // No route serves this URL, so there is no target to point at.
            <li><a href="/no/such/page">"An unrouted URL (404)"</a></li>
        </ul>
    })
}

// An error keeps its type on the way out, so an error boundary around the slot
// can downcast it and replace the page with a branded error page.
#[layout]
async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <html>
            <body>
                error_boundary(
                    fallback: |error| {
                        let (status_code, heading) = if error
                            .downcast_ref::<NotFoundError>()
                            .is_some() {
                            (StatusCode::NOT_FOUND, "Page not found")
                        } else if error.downcast_ref::<ForbiddenError>().is_some() {
                            (StatusCode::FORBIDDEN, "Access denied")
                        } else {
                            // Any other error is rethrown for the handler to answer.
                            return Err(error);
                        };

                        Ok(
                            view! {
                                (status_code)
                                <h1>(heading)</h1>
                            },
                        )
                    },
                    (slot)
                )
                <p><a href=(href!(home))>"Home"</a></p>
            </body>
        </html>
    })
}

// A URL matching no route is normally answered with a bare 404 that skips the
// layouts. This catch-all page resolves such URLs to a NotFoundError instead,
// so the layout brands them like any other handler error.
not_found!();
