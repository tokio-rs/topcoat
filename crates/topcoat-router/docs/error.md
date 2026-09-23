Router errors, and how handler errors become HTTP responses.

Every page, layout, layer, and route handler returns a `Result`. When it returns an `Err`, the error becomes the response. Each error type in this module maps to an HTTP status code. Any other error becomes a `500 Internal Server Error`.

# Constructors

Each error type in this module has a constructor function named after the response it produces. For example, [`not_found()`](not_found) responds 404 with a [`NotFoundError`], [`redirect(uri)`](redirect) responds 307 with a [`RedirectError`], and [`bad_request(description)`](bad_request) responds 400 with a [`BadRequestError`] and a description that is safe to show to the client. [`too_many_requests(secs)`](too_many_requests) and [`service_unavailable(secs)`](service_unavailable) respond 429 and 503, and both set a `Retry-After` header. [`see_other(uri)`](see_other) responds 303 with a [`SeeOther`]. `SeeOther` is also a normal response, so a route can return it through `Ok` as well.

A constructor returns a concrete error type that converts into the handler's error type. Return it with `?` or convert it with `.into()`:

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

The router also creates some of these errors itself. A request that matches no route gets a [`NotFoundError`]. A request whose path matches but whose method does not gets a [`MethodNotAllowedError`]. A request body that fails to parse gets a [`BadRequestError`], and a request body larger than the body limit gets a [`ContentTooLargeError`].

# From an `Option` or `Result`

Often the error comes from a missing value. [`RouterErrorExt`] adds `ok_or_*` methods to [`Option`] and [`core::result::Result`]. They replace `None`, or any `Err`, with a router error that you can return with `?`:

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

Each method matches a constructor. [`ok_or_not_found`](RouterErrorExt::ok_or_not_found) matches [`not_found`], [`ok_or_redirect`](RouterErrorExt::ok_or_redirect) matches [`redirect`], and so on. The `error = ...` option of `path_param!` and `#[query_params]` uses the same constructors when parsing fails.

# Catching an error

An error keeps its type as it travels up, so an outer handler can check it with `downcast_ref` and show a view instead. Wrap the content that can fail in an [`error_boundary`](https://docs.rs/topcoat/latest/topcoat/view/struct.error_boundary.html). When the content fails, the boundary passes the error to its fallback and shows the view that the fallback returns. For example, a layout can replace a [`ForbiddenError`] from any page below it with a custom access-denied page:

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
                            // Any other error type is passed on.
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

The [`StatusCode`](crate::StatusCode) in the view keeps the response status at 403. Without it, the replacement page would be sent with status 200. When the fallback returns an error, that error keeps moving up the view tree.

You can also handle errors with [`live!`](https://docs.rs/topcoat/latest/topcoat/view/macro.live.html) regions instead of the simpler `error_boundary` component.

# Not-found pages

A [`NotFoundError`] returned by a handler can be caught in the same way. The 404 for a URL that matches no route is different. Only layers whose path is `None` run for it, because they wrap every request. No other layer and no layout runs for a request that matches nothing. To render those URLs inside your layouts as well, declare a catch-all page with [`not_found!`](https://docs.rs/topcoat/latest/topcoat/router/macro.not_found.html):

```rust
# use topcoat::router::not_found;
not_found!("/");
```

This declares a page that answers every otherwise unmatched URL under its path with a [`NotFoundError`]. The error then moves up through the layouts like any other handler error. See the [`not_found!` reference](https://docs.rs/topcoat/latest/topcoat/router/macro.not_found.html) for the module-derived form and for how the catch-all segment is added.

# Rewrites

A rewrite handles the request again under a different path. The whole route stack runs as if the client had requested that path. Unlike a redirect, the client does not see a rewrite. The URL in the browser stays the same, and there is no extra round trip. Create one with [`rewrite(path, body)`](rewrite) and return it like any other router error:

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

The rewritten request keeps the method and headers of the original request and uses `body` as its body. The path can include a query string. Everything else starts fresh. The response built so far is thrown away together with the request context, so per-request state, such as memoized values or response cookies set by the abandoned attempt, does not carry over. All layers run again, including layers without a path.

The returned [`RewriteError`] has more options. [`method`](RewriteError::method) sends the rewritten request with a different HTTP method. [`with`](RewriteError::with) passes a value to the next dispatch and to every later dispatch in the same chain, where [`request_context`](topcoat_core::context::request_context) can read it. Each dispatch still gets a fresh context and memoization cache. Calling `with` again, or on a later rewrite, replaces a passed value of the same type and keeps the others. A form handler can use both options to render its page as a `GET`, with a value that tells the page what happened:

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

A handler reached through a rewrite sees the rewritten request in [`parts`](crate::request::parts) and the accessors for its fields. To read the request as the client sent it, for example to get the URL a form should post back to, use [`original_parts`](crate::request::original_parts) or an accessor such as [`original_uri`](crate::request::original_uri) or [`original_method`](crate::request::original_method).

The router refuses a rewrite to a path the request was already handled under, and it stops a chain after 8 rewrites. Both cases respond with a 500 and do not reveal the chain to the client.

# Unexpected errors

Any other error responds with a 500 and does not reveal its message to the client. When you need such an error as a typed value, wrap it in an [`InternalServerError`] with [`internal_server_error(error)`](internal_server_error). The response stays the same.
