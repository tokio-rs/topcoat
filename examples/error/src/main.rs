use topcoat::{
    Result,
    context::Cx,
    router::{
        Router, RouterBuilderDiscoverExt, StatusCode,
        error::{ForbiddenError, NotFoundError, RouterErrorExt, forbidden},
        layout, not_found, page, path_param,
    },
    view::{ViewHandle, view},
};

#[tokio::main]
async fn main() {
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

#[page("/")]
async fn home() -> Result {
    view! {
        <h1>"Error handling"</h1>
        <ul>
            <li><a href="/posts/1">"An existing post"</a></li>
            <li><a href="/posts/7">"A missing post (404)"</a></li>
            <li><a href="/admin">"The admin area (403)"</a></li>
            <li><a href="/no/such/page">"An unrouted URL (404)"</a></li>
        </ul>
    }
}

// An error keeps its type on the way out, so the layout can downcast it and
// replace it with a branded error page. The slot is the page's reactive view
// handle; `live match` consumes it and catches the error in place.
#[layout("/")]
async fn root_layout(slot: ViewHandle<'_>) -> Result {
    view! {
        <html>
            <body>
                live match slot {
                    Err(error) if error.downcast_ref::<NotFoundError>().is_some() => {
                        (StatusCode::NOT_FOUND)
                        <h1>"Page not found"</h1>
                    }
                    Err(error) if error.downcast_ref::<ForbiddenError>().is_some() => {
                        (StatusCode::FORBIDDEN)
                        <h1>"Access denied"</h1>
                    }
                    other => {
                        // The rethrow is a plain `?`: anything else fails this
                        // construct and climbs to the next catcher out.
                        (other?)
                    }
                }
                <p><a href="/">"Home"</a></p>
            </body>
        </html>
    }
}

path_param!(post_id: u64, error = bad_request);

// ok_or_not_found turns the None into a 404, which the layout catches above.
#[page("/posts/{post_id}")]
async fn post(cx: &Cx) -> Result {
    let title = match *path_param::<PostId>(cx)? {
        1 => Some("Hello Topcoat"),
        2 => Some("Error handling"),
        _ => None,
    }
    .ok_or_not_found()?;

    view! { <h1>(title)</h1> }
}

// An error constructor converts into the handler's error type.
#[page("/admin")]
async fn admin() -> Result {
    Err(forbidden().into())
}

// A URL matching no route is normally answered with a bare 404 that skips the
// layouts. This catch-all page resolves such URLs to a NotFoundError instead,
// so the layout brands them like any other handler error.
not_found!("/");
