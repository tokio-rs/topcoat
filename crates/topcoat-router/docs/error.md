Turning handler errors into HTTP responses.

Every page, layout, layer, and route handler returns a `Result`. An `Err` becomes the response: the router maps each of its own error types onto an HTTP status code and turns anything else into a 500.

# Constructors

Every error type in this module has a constructor function named after its response. For example, [`not_found()`](not_found) responds 404 with [`NotFoundError`], [`redirect(uri)`](redirect) responds 307 with [`RedirectError`], and [`bad_request(description)`](bad_request) responds 400 with [`BadRequestError`] and a client-safe description.

A constructor returns a concrete error type that converts into the handler's error, so bubble it up with `?` or return it directly:

```rust
use topcoat::{Result, context::Cx, router::{error::not_found, page}, view::view};
# struct Post;
# async fn find_post(_cx: &Cx) -> Option<Post> { None }
#[page("/posts/{id}")]
async fn post(cx: &Cx) -> Result {
    let Some(_post) = find_post(cx).await else {
        return Err(not_found().into());
    };
    view! { <h1>"Post"</h1> }
}
```

The router raises some of these itself: a request that matches no route gets a [`NotFoundError`], a matched path with the wrong method a [`MethodNotAllowedError`], and a request body that fails to parse a [`BadRequestError`].

# From an `Option` or `Result`

Usually the failing value is the condition. [`RouterErrorExt`] adds `ok_or_*` methods to [`Option`] and [`core::result::Result`] that replace `None` (or any `Err`) with a router error, ready for `?`:

```rust
# use topcoat::{Result, context::Cx, router::{error::RouterErrorExt, page}, view::view};
# struct User;
# async fn current_session(_cx: &Cx) -> Option<User> { None }
#[page("/dashboard")]
async fn dashboard(cx: &Cx) -> Result {
    let _user = current_session(cx).await.ok_or_unauthorized()?;
    view! { <h1>"Dashboard"</h1> }
}
```

The methods mirror the constructors: [`ok_or_not_found`](RouterErrorExt::ok_or_not_found) for [`not_found`], [`ok_or_redirect`](RouterErrorExt::ok_or_redirect) for [`redirect`], and so on. A failed `#[path_param]` or `#[query_params]` parse feeds the same constructors through the macro's `error = ...` option.

# Catching an error

An error keeps its type on the way out, so an outer handler can pick it up with `downcast_ref` and respond with a view instead. For example, a layout can replace a page's [`NotFoundError`] with a branded not-found page:

```rust
use topcoat::{
    Result,
    router::{StatusCode, error::NotFoundError, layout},
    view::view,
};

#[layout("/")]
async fn root_layout(slot: Result) -> Result {
    let content = match slot {
        Err(error) if error.downcast_ref::<NotFoundError>().is_some() => view! {
            (StatusCode::NOT_FOUND)
            <h1>"Page not found"</h1>
        },
        content => content,
    }?;

    view! {
        <html>
            <body>(content)</body>
        </html>
    }
}
```

The [`StatusCode`](crate::StatusCode) in the view keeps the response a 404; without it the replacement page would be served as a 200.

# Unexpected errors

Any other error responds 500 without leaking its message to the client. To record a source error while keeping that behavior, wrap it in [`internal_server_error(error)`](internal_server_error) yourself.
