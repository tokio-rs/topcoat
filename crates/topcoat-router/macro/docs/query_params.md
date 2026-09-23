Declares a typed view of the request's query string.

Apply `#[query_params]` to a struct with named fields. The macro derives [`serde::Deserialize`](https://docs.rs/serde/latest/serde/trait.Deserialize.html), so each field maps to a key in the query string.

```rust
# use topcoat::router::query_params;
#[query_params]
struct PageQuery {
    page: Option<u32>,
}
```

# Reading the value

Read the struct with [`query_params::<T>(cx)`](fn.query_params.html). It returns `Result<&T, &QueryParamsError>` and caches the result for the request. A [`QueryParamsError`](type.QueryParamsError.html) identifies the field that failed to parse. The struct is independent of the route, so any handler can read it.

# Failing with an error response

Set `error = ...` to convert parse failures into a router error. Callers can then propagate the error with `?`:

```rust
# use topcoat::{context::Cx, Result, router::{page, query_params}, view::{View, view}};
#[query_params(error = bad_request)]
struct PageQuery {
    page: Option<u32>,
}

#[page("/posts")]
async fn posts(cx: &Cx) -> Result<impl View> {
    // Responds with a 400 naming the failing key when the query string
    // does not match.
    let query = query_params::<PageQuery>(cx)?;
    Ok(view! { "currently on page: " (query.page) })
}
```

The forms mirror the router's error constructors:

- `error = not_found` responds 404 with [`NotFoundError`](error/struct.NotFoundError.html).
- `error = unauthorized` responds 401 with [`UnauthorizedError`](error/struct.UnauthorizedError.html).
- `error = forbidden` responds 403 with [`ForbiddenError`](error/struct.ForbiddenError.html).
- `error = bad_request` responds 400 with [`BadRequestError`](error/struct.BadRequestError.html). The description names the failing key, like ``invalid query value: invalid digit found in string (at `page`)``; pass your own with `error = bad_request("invalid search")`.
- `error = redirect("/search")` and `error = redirect_permanent("/search")` send the client to the given URI with [`RedirectError`](error/struct.RedirectError.html).

Without `error = ...`, the same conversions are available per call site through [`RouterErrorExt`](error/trait.RouterErrorExt.html), which also suits handlers that want different responses for the same struct.

## Clearing the query string with `redirect("?")`

The client resolves a redirect target against the current URL. The target `"?"` reloads the current page with an empty query string:

```rust
# use topcoat::router::query_params;
#[query_params(error = redirect("?"))]
struct PageQuery {
    page: Option<u32>,
}
```

Use this only when an empty query string is valid. Otherwise the redirected request fails again and creates a redirect loop.

# Requirements

- Use `Option<T>` for optional keys. A missing key and an empty value (`?page=`, as a browser sends for a blank input) both read as `None`. `#[serde(default)]` is not applied, so a missing key on a non-`Option` field is a parse error.
- The struct must be `Send + Sync + 'static`, since it is memoized for the request.
