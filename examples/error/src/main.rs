use topcoat::{
    Result,
    context::Cx,
    router::{
        Body, Router, RouterBuilderDiscoverExt, Slot, StatusCode,
        error::{ForbiddenError, NotFoundError, RouterErrorExt, forbidden, rewrite},
        href, layout, not_found, page, path_param,
    },
    view::{View, error_boundary, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

#[page("/")]
async fn home() -> Result<impl View> {
    Ok(view! {
        <h1>"Error handling"</h1>
        <ul>
            <li><a href=(href!(post, PostId(1)))>"An existing post"</a></li>
            <li><a href=(href!(post, PostId(7)))>"A missing post (404)"</a></li>
            <li><a href=(href!(admin))>"The admin area (403)"</a></li>
            <li><a href=(href!(rewrite_page))>"A page rewrite"</a></li>
            // No route serves this URL, so there is no target to point at.
            <li><a href="/no/such/page">"An unrouted URL (404)"</a></li>
        </ul>
    })
}

// An error keeps its type on the way out, so an error boundary around the slot
// can downcast it and replace the page with a branded error page.
#[layout("/")]
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

path_param!(post_id: u64, error = bad_request);

// ok_or_not_found turns the None into a 404, which the error handler catches above.
#[page("/posts/{post_id}")]
async fn post(cx: &Cx) -> Result<impl View> {
    let title = match *path_param::<PostId>(cx)? {
        1 => Some("Hello Topcoat"),
        2 => Some("Error handling"),
        _ => None,
    }
    .ok_or_not_found()?;

    Ok(view! { <h1>(title)</h1> })
}

// An error constructor converts into the handler's error type.
#[page("/admin")]
async fn admin() -> Result<()> {
    Err(forbidden().into())
}

// A rewrite starts a new server-side dispatch without changing the browser URL.
#[page("/rewrite")]
async fn rewrite_page() -> Result<()> {
    Err(rewrite("/rewritten", Body::empty()).into())
}

#[page("/rewritten")]
async fn rewritten() -> Result<impl View> {
    Ok(view! { <h1>"The rewrite target"</h1> })
}

// A URL matching no route is normally answered with a bare 404 that skips the
// layouts. This catch-all page resolves such URLs to a NotFoundError instead,
// so the layout brands them like any other handler error.
not_found!("/");
