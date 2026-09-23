Use router errors to return an HTTP error response from a handler. An unhandled error of another type becomes `500 Internal Server Error`.

# Constructors

Call a constructor such as [`not_found()`](not_found) and convert its error with `.into()`. You can also propagate these errors with `?`:

```rust
use topcoat::{Result, context::Cx, router::{error::not_found, page}, view::{View, view}};
# struct Post;
# async fn find_post(_cx: &Cx) -> Option<Post> { None }
#[page("/posts/{id}")]
async fn post(cx: &Cx) -> Result<impl View> {
    let Some(_post) = find_post(cx).await else {
        return Err(not_found().into());
    };
    Ok(view! { <h1>"Post"</h1> })
}
```

# From an `Option` or `Result`

[`RouterErrorExt`] adds methods that replace `None` or an `Err` with a router error. For example, reject a request without a session:

```rust
# use topcoat::{Result, context::Cx, router::{error::RouterErrorExt, page}, view::{View, view}};
# struct User;
# async fn current_session(_cx: &Cx) -> Option<User> { None }
#[page("/dashboard")]
async fn dashboard(cx: &Cx) -> Result<impl View> {
    let _user = current_session(cx).await.ok_or_unauthorized()?;
    Ok(view! { <h1>"Dashboard"</h1> })
}
```

Choose the method for the response you want to send. These conversions replace the original error, so handle or record it first if you need to keep it.

# Catching an error

Wrap content in an [`error_boundary`](https://docs.rs/topcoat/latest/topcoat/view/struct.error_boundary.html) to show a replacement view when it fails. Its fallback receives the error and can inspect its type with `downcast_ref`. For example, a layout can show an access-denied page for [`ForbiddenError`]:

```rust
use topcoat::{
    Result,
    router::{Slot, StatusCode, error::ForbiddenError, layout},
    view::{View, error_boundary, view},
};

#[layout("/")]
async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <html>
            <body>
                error_boundary(
                    fallback: |error| {
                        if error.downcast_ref::<ForbiddenError>().is_none() {
                            // Any other error type is rethrown.
                            return Err(error);
                        }

                        Ok(view! {
                            (StatusCode::FORBIDDEN)
                            <h1>"Access denied"</h1>
                        })
                    },
                    (slot)
                )
            </body>
        </html>
    })
}
```

The [`StatusCode`](crate::StatusCode) in the view keeps the response a 403; without it the replacement page would be served as a 200. Returning an error from the fallback rethrows it; it will continue bubbling up the view tree.

# Not-found pages

A [`NotFoundError`] returned by a handler is caught the same way. The 404 for a URL matching no route reaches only layers whose path is `None`, since they wrap every request; no other layer or layout runs for a request nothing was registered for. To render those URLs through the layouts with the same branded treatment, declare a catch-all page with [`not_found!`](../macro.not_found.html):

```rust
# use topcoat::router::not_found;
not_found!("/");
```

This registers a page resolving every otherwise unmatched URL under its path to a [`NotFoundError`], which then bubbles through the layouts like any other handler error. See the [`not_found!` reference](../macro.not_found.html) for the module-derived form and how the catch-all segment is appended.

# Rewrites

A rewrite dispatches the request again at a different path, running the whole route stack as if that path had been requested in the first place. Unlike a redirect it is invisible to the client: the browser URL stays what was requested, and no extra round trip happens. Build one with [`rewrite(path, body)`](rewrite) and return it like any other router error:

```rust
use topcoat::{Result, context::Cx, router::{Body, error::rewrite, page}, view::{View, view}};
# async fn beta_tester(_cx: &Cx) -> bool { false }
#[page("/dashboard")]
async fn dashboard(cx: &Cx) -> Result<impl View> {
    if beta_tester(cx).await {
        return Err(rewrite("/dashboard-beta", Body::empty()).into());
    }
    Ok(view! { <h1>"Dashboard"</h1> })
}
```

The rewritten dispatch keeps the request's method and headers and reads `body` as its request body; the path may carry a query string. The new dispatch gets a fresh request context and discards the previous response. Layers run again, including pathless ones. Pass state explicitly with [`RewriteError::with`] when it must survive a rewrite.

The returned [`RewriteError`] has more configuration options. [`method`](RewriteError::method) dispatches the rewritten request with a different HTTP method than the one it arrived with. [`with`](RewriteError::with) carries a value into the next dispatch and every later dispatch in the same chain, where it is available through [`request_context`](topcoat_core::context::request_context). Each dispatch still gets a fresh context and memoization cache. Calling `with` again, or on a later rewrite, replaces a carried value of the same type while keeping the other carried values. A form handler can combine both options to render its page as a `GET` with a value telling the page what happened:

```rust
use topcoat::{Result, context::{Cx, try_request_context}, router::{Body, Method, error::rewrite, page, route}, view::{View, view}};

struct Saved;

#[route(POST "/settings/save")]
async fn save_settings() -> Result<()> {
    Err(rewrite("/settings", Body::empty())
        .method(Method::GET)
        .with(Saved)
        .into())
}

#[page("/settings")]
async fn settings(cx: &Cx) -> Result<impl View> {
    let saved = try_request_context::<Saved>(cx).is_some();
    Ok(view! {
        if saved {
            <p>"Settings saved."</p>
        }
        <form method="post" action="/settings/save"></form>
    })
}
```

A handler reached through a rewrite sees the rewritten request in [`parts`](crate::request::parts) and its field accessors. To read the request as the client actually sent it, for example the URL a form should post back to, use [`original_parts`](crate::request::original_parts) or a field accessor like [`original_uri`](crate::request::original_uri) and [`original_method`](crate::request::original_method).

The router refuses a rewrite to a path the request was already dispatched under and stops any chain after 8 rewrites; both respond 500 without leaking the chain to the client.

# Unexpected errors

Any other error responds 500 without leaking its message to the client. To record a source error while keeping that behavior, wrap it in [`internal_server_error(error)`](internal_server_error) yourself.
