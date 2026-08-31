Turning handler errors into HTTP responses.

Every page, layout, layer, and route handler returns a `Result`. An `Err` becomes the response: the router maps each of its own error types onto an HTTP status code and turns anything else into a 500.

# Constructors

Every error type in this module has a constructor function named after its response. For example, [`not_found()`](not_found) responds 404 with [`NotFoundError`], [`redirect(uri)`](redirect) responds 307 with [`RedirectError`], and [`bad_request(description)`](bad_request) responds 400 with [`BadRequestError`] and a client-safe description. [`too_many_requests(secs)`](too_many_requests) and [`service_unavailable(secs)`](service_unavailable) respond 429 and 503, each carrying a `Retry-After` header.

A constructor returns a concrete error type that converts into the handler's error, so bubble it up with `?` or return it directly:

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

The router raises some of these itself: a request that matches no route gets a [`NotFoundError`], a matched path with the wrong method a [`MethodNotAllowedError`], a request body that fails to parse a [`BadRequestError`], and a request body over the body limit a [`ContentTooLargeError`].

# From an `Option` or `Result`

Usually the failing value is the condition. [`RouterErrorExt`] adds `ok_or_*` methods to [`Option`] and [`core::result::Result`] that replace `None` (or any `Err`) with a router error, ready for `?`:

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

The methods mirror the constructors: [`ok_or_not_found`](RouterErrorExt::ok_or_not_found) for [`not_found`], [`ok_or_redirect`](RouterErrorExt::ok_or_redirect) for [`redirect`], and so on. A failed `path_param::<T>(cx)` or `query_params::<T>(cx)` parse feeds the same constructors through the declaration's `error = ...` option.

# Catching an error

An error keeps its type on the way out, so an outer handler can pick it up with `downcast_ref` and respond with a view instead. For example, a layout can replace a [`ForbiddenError`] bubbling out of any page below it with a branded access-denied page:

```rust
use topcoat::{
    Result,
    router::{Slot, StatusCode, error::ForbiddenError, layout},
    view::{View, emit, live, view},
};

#[layout("/")]
async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <html>
            <body>
                (live! {
                    match emit! { (slot) } {
                        Err(error) if error.downcast_ref::<ForbiddenError>().is_some() => emit! {
                            (StatusCode::FORBIDDEN)
                            <h1>"Access denied"</h1>
                        },
                        slot => slot,
                    }
                })
            </body>
        </html>
    })
}
```

The [`StatusCode`](crate::StatusCode) in the view keeps the response a 403; without it the replacement page would be served as a 200.

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

The rewritten dispatch keeps the request's method and headers and reads `body` as its request body; the path may carry a query string. Everything else starts over: the response built so far is discarded along with the request context, so per-request state like memoized values or response cookies staged by the abandoned dispatch does not leak into the new one. Layers run again too, including pathless ones.

A handler reached through a rewrite sees the rewritten URI in [`uri`](crate::request::uri). To read the URL the client actually requested, for example as a form's post-back target, use [`original_uri`](crate::request::original_uri).

The router refuses a rewrite to a path the request was already dispatched under and stops any chain after 8 rewrites; both respond 500 without leaking the chain to the client.

# Unexpected errors

Any other error responds 500 without leaking its message to the client. To record a source error while keeping that behavior, wrap it in [`internal_server_error(error)`](internal_server_error) yourself.
