Turning handler errors into HTTP responses.

Handlers return a `Result`. An unhandled router error selects an HTTP error or redirect response. Other errors produce `500 Internal Server Error`.

# Constructors

Construct an error with the function named for the response you want. For example, [`not_found()`](not_found) returns a [`NotFoundError`] for a 404 response. [`bad_request(description)`](bad_request) returns a 400 response and exposes the description to the client, so use a client-safe message. See each constructor for its status code and headers.

A constructor returns a concrete error type that converts into the handler's error, so bubble it up with `?` or return it directly:

```rust
// src/app/posts/id.rs
use topcoat::{Result, context::Cx, router::{module_param, error::not_found, page}, view::{View, view}};
# struct Post;
# async fn find_post(_cx: &Cx) -> Option<Post> { None }
module_param!(id);

#[page]
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
// src/app/dashboard.rs
# use topcoat::{Result, context::Cx, router::{error::RouterErrorExt, page}, view::{View, view}};
# struct User;
# async fn current_session(_cx: &Cx) -> Option<User> { None }
#[page]
async fn dashboard(cx: &Cx) -> Result<impl View> {
    let _user = current_session(cx).await.ok_or_unauthorized()?;
    Ok(view! { <h1>"Dashboard"</h1> })
}
```

The methods mirror the constructors: [`ok_or_not_found`](RouterErrorExt::ok_or_not_found) for [`not_found`], [`ok_or_redirect`](RouterErrorExt::ok_or_redirect) for [`redirect`], and so on. A failed `path_param::<T>(cx)` or `query_params::<T>(cx)` parse feeds the same constructors through the declaration's `error = ...` option.

# Catching an error

Wrap content in an [`error_boundary`](https://docs.rs/topcoat/latest/topcoat/view/struct.error_boundary.html) to render a fallback when it fails. The fallback receives the error and can inspect its type with `downcast_ref`. For example, a layout can catch a [`ForbiddenError`] from its page and render an access-denied view:

```rust
// src/app.rs
use topcoat::{
    Result,
    router::{Slot, StatusCode, error::ForbiddenError, layout},
    view::{View, error_boundary, view},
};

#[layout]
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
not_found!();
```

This registers a page resolving every otherwise unmatched URL under its path to a [`NotFoundError`], which then bubbles through the layouts like any other handler error. See the [`not_found!` reference](../macro.not_found.html) for the module-derived form and how the catch-all segment is appended.

# Rewrites

Use a rewrite to handle the request at another path on the server. The router runs that path's layers and handler. The browser URL stays the same, and the client does not send another request. Return [`rewrite(path, body)`](rewrite) as an error:

```rust
// src/app/dashboard.rs
use topcoat::{Result, context::Cx, router::{Body, error::rewrite, page}, view::{View, view}};
# async fn beta_tester(_cx: &Cx) -> bool { false }
#[page]
async fn dashboard(cx: &Cx) -> Result<impl View> {
    if beta_tester(cx).await {
        return Err(rewrite("/dashboard-beta", Body::empty()).into());
    }
    Ok(view! { <h1>"Dashboard"</h1> })
}
```

By default, the rewritten request keeps its method and headers. It uses the body you supply, and its target can include a different query string. The router creates a fresh context and memoization cache, drops pending changes to the previous response, and runs all matching layers again. Layers without a path run again too.

Set [`method`](RewriteError::method) to change the HTTP method, or [`headers`](RewriteError::headers) to replace the headers. If you only need to edit a few headers, copy the current [`headers`](crate::request::headers) first. For example, when replacing the body with an empty one, you can remove headers describing the old body.

Pass a value to [`with`](RewriteError::with) to keep it in the rewritten request and any rewrites that follow. The handler reads it with [`context`](topcoat_core::context::context). Passing the same type again replaces its value without removing the others. In this example, a form handler rewrites to its page with method `GET` and a `Saved` value so the page can show a confirmation:

```rust
use topcoat::{Result, context::{Cx, try_context}, router::{Body, Method, error::rewrite, href, page, route}, view::{View, view}};

struct Saved;

mod settings {
    use super::*;

    mod save {
        use super::*;
    
        #[route(POST)]
        pub(super) async fn save_settings(cx: &Cx) -> Result<()> {
            Err(rewrite(href!(super::settings).resolve(cx), Body::empty())
                .method(Method::GET)
                .with(Saved)
                .into())
        }
    }
    
    #[page]
    async fn settings(cx: &Cx) -> Result<impl View> {
        let saved = try_context::<Saved>(cx).is_some();
        Ok(view! {
            if saved {
                <p>"Settings saved."</p>
            }
            <form method="post" action=(href!(save::save_settings))></form>
        })
    }
}
# fn main() {}
```

After a rewrite, [`parts`](crate::request::parts) and the other request helpers read the rewritten request. To read what the client sent, use [`original_parts`](crate::request::original_parts), [`original_uri`](crate::request::original_uri), or [`original_method`](crate::request::original_method). The original URI is useful when a form should submit back to the URL the client opened.

The router stops a rewrite chain if it reaches the same method, path, and query twice. Rewriting from `POST` to `GET` at the same URL is allowed because the method differs. A repeated request or more than 8 rewrites produces a 500 response. The response does not reveal the rewrite history.

# Unexpected errors

Any other error responds 500 without leaking its message to the client. To record a source error while keeping that behavior, wrap it in [`internal_server_error(error)`](internal_server_error) yourself.
