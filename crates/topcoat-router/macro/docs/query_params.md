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

[`query_params::<T>(cx)`](fn.query_params.html) parses the current request's query string with [`serde_urlencoded`](https://docs.rs/serde_urlencoded/latest/serde_urlencoded/) and returns `Result<&T, &QueryParamsError>`: a reference to the parsed struct, or to the [`QueryParamsError`](type.QueryParamsError.html) naming the key that failed. Unlike a path parameter, the struct is not tied to a route: any handler can read it. Parsing runs at most once per request; the result is then memoized.

# Failing with an error response

A query string that fails to parse is usually answered with a user-facing error response. Declare that response once on the struct with `error = ...`, and the `Err` side of the `Result` becomes the corresponding router error, ready to be bubbled up with `?`:

```rust
# use topcoat::{context::Cx, Result, router::{page, query_params}, view::view};
#[query_params(error = bad_request)]
struct PageQuery {
    page: Option<u32>,
}

#[page("/posts")]
async fn posts(cx: &Cx) -> Result {
    // Responds with a 400 naming the failing key when the query string
    // does not match.
    let query = query_params::<PageQuery>(cx)?;
    view! { "currently on page: " (query.page) }
}
```

The forms mirror the router's error constructors:

- `error = not_found` responds 404 with [`NotFoundError`](struct.NotFoundError.html).
- `error = unauthorized` responds 401 with [`UnauthorizedError`](struct.UnauthorizedError.html).
- `error = forbidden` responds 403 with [`ForbiddenError`](struct.ForbiddenError.html).
- `error = bad_request` responds 400 with [`BadRequestError`](struct.BadRequestError.html). The description names the failing key, like ``invalid query value: invalid digit found in string (at `page`)``; pass your own with `error = bad_request("invalid search")`.
- `error = redirect("/search")` and `error = redirect_permanent("/search")` send the client to the given URI with [`RedirectError`](struct.RedirectError.html).

Without `error = ...`, the same conversions are available per call site through [`RouterErrorExt`](trait.RouterErrorExt.html), which also suits handlers that want different responses for the same struct.

## Clearing the query string with `redirect("?")`

Redirect targets are URI references that the client resolves against the current URL, so relative targets work too. In particular, `"?"` is the current page with an empty query string: instead of failing the request, a query that does not parse reloads the page without one, and the error disappears.

```rust
# use topcoat::router::query_params;
#[query_params(error = redirect("?"))]
struct PageQuery {
    page: Option<u32>,
}
```

This relies on every field being optional; a required key would still be missing after the redirect and loop.

# Requirements

- Use `Option<T>` for optional keys. `serde_urlencoded` does not apply `#[serde(default)]`, so a missing key on a non-`Option` field is a parse error.
- The struct must be `Send + Sync + 'static`, since it is memoized for the request.
