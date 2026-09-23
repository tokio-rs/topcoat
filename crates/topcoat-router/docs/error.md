Turning handler errors into HTTP responses.

Handlers return a `Result`. An unhandled router error selects an HTTP error or redirect response. Other errors produce `500 Internal Server Error`.

# Constructors

Construct an error with the function named for the response you want. For example, [`not_found()`](not_found) returns a [`NotFoundError`] for a 404 response. [`bad_request(description)`](bad_request) returns a 400 response and exposes the description to the client, so use a client-safe message. See each constructor for its status code and headers.

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

[`RouterErrorExt`] adds `ok_or_*` methods to [`Option`] and [`core::result::Result`]. They replace `None` or `Err` with a router error that you can propagate with `?`:

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

Wrap content in an [`error_boundary`](https://docs.rs/topcoat/latest/topcoat/view/struct.error_boundary.html) to render a fallback when it fails. The fallback receives the error and can inspect its type with `downcast_ref`. For example, a layout can catch a [`ForbiddenError`] from its page and render an access-denied view:

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

The [`StatusCode`](crate::StatusCode) keeps the response at 403. Without it, the fallback view would produce a 200 response. Return an error from the fallback to let an outer boundary or the router handle it.

You may also use [`live!`](../view/macro.live.html) regions to handle errors instead of using the simpler `error_boundary` component.

# Not-found pages

A [`NotFoundError`] returned by a handler is caught the same way. The 404 for a URL matching no route reaches only layers whose path is `None`, since they wrap every request; no other layer or layout runs for a request nothing was registered for. To render those URLs through the layouts with the same branded treatment, declare a catch-all page with [`not_found!`](../macro.not_found.html):

```rust
# use topcoat::router::not_found;
not_found!("/");
```

This registers a page resolving every otherwise unmatched URL under its path to a [`NotFoundError`], which then bubbles through the layouts like any other handler error. See the [`not_found!` reference](../macro.not_found.html) for the module-derived form and how the catch-all segment is appended.

# Rewrites

A rewrite handles the request again at another path. It runs the layers and handler for that path without changing the browser URL or making another client request. Return [`rewrite(path, body)`](rewrite) as an error:

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

The rewritten request keeps the method and headers, uses the supplied body, and may include a new query string. It gets a fresh request context and memoization cache. Pending response changes from the abandoned dispatch are discarded. All matching layers run again, including pathless layers.

Use [`method`](RewriteError::method) to change the HTTP method and [`headers`](RewriteError::headers) to replace the request headers. To change only a few headers, clone the current [`headers`](crate::request::headers) and edit that copy. For example, a rewrite with an empty body can remove the headers that described the original body.

Use [`with`](RewriteError::with) to carry a value into the next dispatch and every later dispatch in the chain. Read it through [`request_context`](topcoat_core::context::request_context). Calling `with` again with the same type replaces that value while keeping the other carried values. A form handler can use `method` and `with` to render its page as a `GET` with a value telling the page what happened:

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

The router detects a cycle when a rewrite repeats a combination of method, path, and query already handled in the chain. Changing the method from `POST` to `GET` at the same URL is allowed. A cycle or a chain exceeding 8 rewrites produces a 500 response without exposing the dispatch history to the client.

# Unexpected errors

Any other error responds 500 without leaking its message to the client. To record a source error while keeping that behavior, wrap it in [`internal_server_error(error)`](internal_server_error) yourself.
