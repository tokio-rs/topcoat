Declares a typed path parameter.

Apply `#[path_param]` to a single-field tuple struct. The struct name, snake-cased, becomes the parameter's name (`PostId` -> `post_id`), and the inner type decides how the raw URL segment is parsed.

```rust
# use topcoat::router::path_param;
#[path_param]
struct PostId(uuid::Uuid);
```

# Reading the value

[`path_param::<T>(cx)`](fn.path_param.html) returns the parameter. The return type follows the inner type:

- **`str`**: returns the raw segment as a `&str` borrowed from the request (no parsing, cannot fail).
- **Anything else**: parses the segment with [`FromStr`](core::str::FromStr) and returns `Result<&T, &<T as FromStr>::Err>`: a reference to the parsed value, or to the parse error.

Parsing runs at most once per request; the result is then memoized.

# Failing with an error response

A segment that fails to parse is usually answered with a user-facing error response. Declare that response once on the parameter with `error = ...`, and the `Err` side of the `Result` becomes the corresponding router error, ready to be bubbled up with `?`:

```rust
# use topcoat::{context::Cx, Result, router::{page, path_param}, view::view};
#[path_param(error = not_found)]
struct PostId(uuid::Uuid);

#[page("/posts/{post_id}")]
async fn post_page(cx: &Cx) -> Result {
    // Responds with a 404 when the segment is not a valid UUID.
    let post_id = path_param::<PostId>(cx)?;
    view! { "showing post with id: " (post_id.to_string()) }
}
```

The forms mirror the router's error constructors:

- `error = not_found` responds 404 with [`NotFoundError`](struct.NotFoundError.html).
- `error = unauthorized` responds 401 with [`UnauthorizedError`](struct.UnauthorizedError.html).
- `error = forbidden` responds 403 with [`ForbiddenError`](struct.ForbiddenError.html).
- `error = bad_request` responds 400 with [`BadRequestError`](struct.BadRequestError.html). The description defaults to `invalid value for path parameter "post_id"`; pass your own with `error = bad_request("no such post")`.
- `error = redirect("/posts")` and `error = redirect_permanent("/posts")` send the client to the given URI with [`RedirectError`](struct.RedirectError.html).

Without `error = ...`, the same conversions are available per call site through [`RouterErrorExt`](trait.RouterErrorExt.html), which also suits handlers that want different responses for the same parameter:

```rust
# use topcoat::{context::Cx, Result, router::{RouterErrorExt, page, path_param}, view::view};
# #[path_param]
# struct PostId(uuid::Uuid);
# #[page("/posts/{post_id}")]
# async fn post_page(cx: &Cx) -> Result {
let post_id = path_param::<PostId>(cx).ok_or_not_found()?;
# view! { (post_id.to_string()) }
# }
```

# Matching the URL segment

The parameter's name has to line up with a `{name}` segment in the route's URL. How that segment gets there depends on how the route is registered:

- **Explicit path**: write the placeholder yourself, so `PostId` must appear as `{post_id}`:
  `#[page("/posts/{post_id}")]`.
- **[`module_router!`](../router/macro.module_router.html)**: defining a `#[path_param]` inside a module turns that module's own segment into the parameter, so there is no placeholder to write. A `PostId` in `src/app/posts/id.rs` makes the `id` module render as `{post_id}`.

# Examples

## Module router

```rust
// src/app/posts/id.rs: the `id` module becomes `{post_id}` in the URL.
use topcoat::{
    context::Cx,
    Result,
    router::{page, path_param},
    view::view,
};

// A failed parse responds 400 with `invalid value for path parameter "post_id"`.
#[path_param(error = bad_request)]
struct PostId(uuid::Uuid);

#[page]
async fn post_page(cx: &Cx) -> Result {
    let post_id = path_param::<PostId>(cx)?;
    view! { "showing post with id: " (post_id.to_string()) }
}
```

## Borrowed `str`

```rust
# use topcoat::{context::Cx, Result, router::{page, path_param}, view::view};
// A `str` inner type skips parsing and borrows the raw segment.
#[path_param]
struct Slug(str);

#[page("/posts/{slug}")]
async fn show(cx: &Cx) -> Result {
    let slug = path_param::<Slug>(cx); // `&str`
    view! { "slug: " (slug) }
}
```

# Requirements

- The struct must be a tuple struct with exactly one field.
- A non-`str` inner type must implement [`FromStr`](core::str::FromStr), and its parsed `Result` must be `Send + Sync + 'static` so it can be [memoized](../topcoat_core_macro/attr.memoize.html) for the request.
- `error = ...` requires a parsed inner type; a `str` parameter cannot fail.
