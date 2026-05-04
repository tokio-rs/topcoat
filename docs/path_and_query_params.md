# Path and query params

Most pages need to read something out of the URL — the post id in `/posts/{post_id}`, the page number in `?page=2`. Topcoat exposes both as typed Rust structs through two attributes:

- `#[path_param]` — one named segment of the path.
- `#[query_params]` — the request's query string, deserialized as a struct.

Both share the same shape: declare a type, call `T::of(cx)` from any handler, and the parsed value is memoized for the rest of the request.

## Path parameters

Annotate a tuple struct with one unnamed field. The struct name (snake-cased) is the parameter's name; the inner type defines how the raw string is parsed.

```rust
use topcoat::router::path_param;

#[path_param]
struct PostId(uuid::Uuid);
```

`PostId` exposes an associated function `PostId::of(cx)` that returns a borrowed reference to the parsed value. Parsing happens at most once per request — repeated calls are free.

### Pairing with the URL

`#[path_param]` only declares *how* to read a parameter. *Which* segment of the URL carries it depends on which router you use.

**Module router.** The macro also emits a `segment!(kind = Param, rename = "...")` for the enclosing module. The module's URL segment is replaced by the parameter, so a `PostId` defined anywhere under module `app::posts::id` turns that module into `{post_id}` in the URL:

```rust
// src/app/posts/id/mod.rs — the `id` module becomes `{post_id}`.
use topcoat::{
    context::Cx,
    router::{RedirectExt, Result, page, path_param},
    view::view,
};

#[path_param]
struct PostId(uuid::Uuid);

#[page]
async fn post_page(cx: &Cx) -> Result {
    let post_id = PostId::of(cx).as_ref().ok_or_redirect("/invalid-id")?;
    view! { "showing post with id: " (post_id.to_string()) }
}
```

**Regular `Router`.** The page's path string is the source of truth. Include a matching `{...}` placeholder; the snake-cased struct name must equal the placeholder for `of` to find the value:

```rust
#[path_param]
struct PostId(uuid::Uuid);

#[page("/posts/{post_id}")]
async fn post_page(cx: &Cx) -> Result {
    let post_id = PostId::of(cx).as_ref().ok_or_redirect("/invalid-id")?;
    view! { "showing post with id: " (post_id.to_string()) }
}
```

The `segment!` emitted by the macro is inert here — it only matters under the module router.

### What `of` returns

The return type depends on the inner field's type:

- **Inner type is `&str`** — `of(cx)` returns `&Self` directly with the borrowed segment value. No parsing, no `Result`.
- **Any other inner type** — `of(cx)` returns `&Result<Self, <T as FromStr>::Err>`, parsed via `FromStr`. Use `.as_ref()` to inspect the result.

```rust
// Borrowed inner type — no parsing.
#[path_param]
struct Slug<'a>(&'a str);

#[page]
async fn show(cx: &Cx) -> Result {
    let slug = Slug::of(cx); // &Slug<'_>
    view! { "slug: " (&**slug) }
}
```

A `Deref` impl to the inner type is also generated, so `&**slug` (or `*post_id`) gives you the raw value when you need it.

## Query parameters

Annotate a struct with named fields. The macro derives `serde::Deserialize` on it and generates `T::of(cx)` that parses the request's query string with `serde_urlencoded`.

```rust
use topcoat::{
    context::Cx,
    router::{Result, page, query_params},
    view::view,
};

#[query_params]
struct PageQuery {
    page: Option<u32>,
}

#[page]
async fn posts(cx: &Cx) -> Result {
    // For `/posts?page=2`, this yields `Some(2)`.
    let q = PageQuery::of(cx).as_ref().unwrap();
    view! {
        <div>
            "currently on page: " (q.page)
        </div>
    }
}
```

`of` returns `&Result<Self, serde_urlencoded::de::Error>`, parsed once per request and shared across calls.

The struct isn't tied to any particular route — define it once, read it from any page, layout, or component that has access to a `Cx`.

### Optional fields

`serde_urlencoded` does **not** apply `#[serde(default)]` automatically. Wrap optional parameters in `Option<T>` to make them tolerate missing values:

```rust
#[query_params]
struct Filters {
    q: Option<String>,        // omit -> None
    page: Option<u32>,        // omit -> None
    sort: String,             // omit -> Err
}
```

## Memoization

Both attributes parse lazily and memoize the parsed value in the request context. Calling `PostId::of(cx)` ten times across a layout, a page, and a few components costs one `FromStr::from_str` call — the rest are cache hits. The same goes for `PageQuery::of(cx)`.

This is the same machinery as [`#[memoize]`](./memoization.md), so the parsed type must satisfy its requirements (`Send + Sync + 'static`).

## Requirements

**`#[path_param]`:**

- The item must be a tuple struct with exactly one unnamed field.
- For non-`&str` inner types, the inner type must implement `FromStr`, and the resulting `Result<Self, _>` must be `Send + Sync + 'static`.

**`#[query_params]`:**

- The struct's fields must be deserializable by `serde_urlencoded` (use `Option<T>` for optional parameters).
- The struct must be `Send + Sync + 'static`.
