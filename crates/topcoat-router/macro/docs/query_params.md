Declares a typed view of the request's query string.

Put `#[query_params]` on a struct with named fields. The macro derives [`serde::Deserialize`](https://docs.rs/serde/latest/serde/trait.Deserialize.html) for it, so each field reads the query string key of the same name.

```rust
# use topcoat::router::query_params;
#[query_params]
struct PageQuery {
    page: Option<u32>,
}
```

# Reading the value

[`query_params::<T>(cx)`](fn.query_params.html) parses the query string of the current request. It returns `Result<&T, &QueryParamsError>`: a reference to the parsed struct, or to a [`QueryParamsError`](type.QueryParamsError.html) that names the key that failed. Unlike a path parameter, the struct is not tied to a route, so any handler can read it. The query string is parsed at most once per request, and later calls return the memoized result.

# Failing with an error response

A query string that fails to parse is usually answered with an error response. Declare that response once on the struct with `error = ...`. The `Err` side of the result then becomes that router error, and the handler can return it with `?`:

```rust
# use topcoat::{context::Cx, Result, router::{page, query_params}, view::{View, view}};
#[query_params(error = bad_request)]
struct PageQuery {
    page: Option<u32>,
}

#[page("/posts")]
async fn posts(cx: &Cx) -> Result<impl View> {
    // Responds with a 400 that names the failing key when the query string
    // does not match.
    let query = query_params::<PageQuery>(cx)?;
    Ok(view! { "currently on page: " (query.page) })
}
```

The supported forms match the router's error constructors:

- `error = not_found` responds 404 with [`NotFoundError`](error/struct.NotFoundError.html).
- `error = unauthorized` responds 401 with [`UnauthorizedError`](error/struct.UnauthorizedError.html).
- `error = forbidden` responds 403 with [`ForbiddenError`](error/struct.ForbiddenError.html).
- `error = bad_request` responds 400 with [`BadRequestError`](error/struct.BadRequestError.html). The response names the failing key, as in ``invalid query value: invalid digit found in string (at `page`)``. Pass your own description with `error = bad_request("invalid search")`.
- `error = redirect("/search")` and `error = redirect_permanent("/search")` redirect the client to the given URI with [`RedirectError`](error/struct.RedirectError.html).

Without `error = ...`, you can make the same conversions at each call site with [`RouterErrorExt`](error/trait.RouterErrorExt.html). This also lets different handlers answer differently for the same struct.

## Clearing the query string with `redirect("?")`

A redirect target is a URI reference, which the client resolves against the current URL, so relative targets work. The target `"?"` means the current page with an empty query string. With `error = redirect("?")`, a query string that does not parse reloads the page without it, instead of failing the request.

```rust
# use topcoat::router::query_params;
#[query_params(error = redirect("?"))]
struct PageQuery {
    page: Option<u32>,
}
```

This only works when every field is optional. A required key would still be missing after the redirect, and the client would be redirected again and again.

# Requirements

- Use `Option<T>` for keys that may be missing. A missing key and an empty value (`?page=`, which a browser sends for a blank input) both read as `None`. `#[serde(default)]` is not added, so a missing key for a field that is not an `Option` is a parse error.
- The struct must be `Send + Sync + 'static`, because it is memoized for the request.
