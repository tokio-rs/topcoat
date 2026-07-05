Declares a typed path parameter.

Apply `#[path_param]` to a tuple struct with a single field. The struct sets up three things:

- **Name**: the struct name, snake-cased, is the parameter's name (`PostId` -> `post_id`).
- **Parsing**: the inner type decides how the raw URL segment is parsed.
- **Access**: read the value with [`path_param::<PostId>(cx)`](fn.path_param.html) from any handler.

```rust
# use topcoat::router::path_param;
#[path_param]
struct PostId(uuid::Uuid);
```

# Reading the value

[`path_param::<T>(cx)`](fn.path_param.html) returns the parameter. The return type follows the inner type:

- **`&str`**: returns the struct directly, borrowing the raw segment (no parsing).
- **Anything else**: returns `Result<&T, &<Inner as FromStr>::Err>`, parsed with [`FromStr`](core::str::FromStr).

Parsing runs at most once per request; the result is then memoized. The struct also [`Deref`](core::ops::Deref)s to its inner type, so you can use the value as if it were that type.

# Matching the URL segment

The parameter's name has to line up with a `{name}` segment in the route's URL. How that segment gets there depends on how the route is registered:

- **Explicit path**: write the placeholder yourself, so `PostId` must appear as `{post_id}`:
  `#[page("/posts/{post_id}")]`.
- **[`module_router!`](../router/macro.module_router.html)**: defining a `#[path_param]` inside a module turns that module's own segment into the parameter, so there is no placeholder to write. A `PostId` in `src/app/posts/id.rs` makes the `id` module render as `{post_id}`.

# Examples

## Explicit path

```rust
# use topcoat::{context::Cx, Result, router::{RouterErrorExt, page, path_param}, view::view};
// The placeholder `{post_id}` matches the snake-cased struct name `PostId`.
#[path_param]
struct PostId(uuid::Uuid);

#[page("/posts/{post_id}")]
async fn post_page(cx: &Cx) -> Result {
    let post_id = path_param::<PostId>(cx).ok_or_redirect("/invalid-id")?;
    view! { "showing post with id: " (post_id.to_string()) }
}
```

## Module router

```rust
// src/app/posts/id.rs: the `id` module becomes `{post_id}` in the URL.
use topcoat::{
    context::Cx,
    Result,
    router::{RouterErrorExt, page, path_param},
    view::view,
};

#[path_param]
struct PostId(uuid::Uuid);

#[page]
async fn post_page(cx: &Cx) -> Result {
    let post_id = path_param::<PostId>(cx).ok_or_redirect("/invalid-id")?;
    view! { "showing post with id: " (post_id.to_string()) }
}
```

## Borrowed `&str`

```rust
# use topcoat::{context::Cx, Result, router::{page, path_param}, view::view};
// A `&str` inner type skips parsing and borrows the raw segment.
#[path_param]
struct Slug<'a>(&'a str);

#[page("/posts/{slug}")]
async fn show(cx: &Cx) -> Result {
    let slug = path_param::<Slug>(cx); // `Slug<'_>`
    view! { "slug: " (&**slug) }
}
```

# Requirements

- The struct must be a tuple struct with exactly one field.
- A non-`&str` inner type must implement [`FromStr`](core::str::FromStr), and its parsed `Result` must be `Send + Sync + 'static` so it can be [memoized](../topcoat_core_macro/attr.memoize.html) for the request.
