Declares a typed view of a path parameter.

Apply this attribute to a tuple struct with a single unnamed field. The struct name, snake-cased, becomes the parameter's name; the inner type defines how the raw string is parsed.

# Pairing with the route's URL

`#[path_param]` only declares how to read a parameter — it does not by itself decide which URL segment carries that parameter. How the param gets into the URL depends on which router you use:

- **Module router** ([`module_router!`](../router/macro.module_router.html)) — the macro also emits a [`segment!`](macro@segment)`(kind = Param, rename = "...")` for the enclosing module. The module's URL segment is replaced by the parameter, so a `PostId` defined anywhere in module `app::posts::id` turns that module into `{post_id}` in the URL.
- **Regular [`Router`](../router/struct.Router.html)** — the page's path string is the source of truth. Include a matching parameter name in the [`#[page("...")]`](macro@page) path; the snake-cased struct name must equal the `{...}` placeholder for `of` to find the value. The [`segment!`](macro@segment) emitted by the macro is inert for this router.

# Reading the parameter

The macro generates an `of(cx: &Cx)` associated function whose return type depends on the inner type:

- **`&str`** — returns `&Self` directly with the borrowed segment value.
- **Any other type** — returns `Result<&Self, &<T as FromStr>::Err>`, parsed via [`FromStr`](core::str::FromStr). Parsing is memoized per request, so repeated calls within a handler do not re-parse.

A [`Deref`](core::ops::Deref) impl to the inner type is also generated.

# Examples

## Module router

```ignore
// src/app/posts/id/mod.rs — the `id` module becomes `{post_id}` in the URL.
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
    let post_id = PostId::of(cx).ok_or_redirect("/invalid-id")?;
    view! { "showing post with id: " (post_id.to_string()) }
}
```

## Regular router

```ignore
// The placeholder `{post_id}` matches the snake-cased struct name `PostId`.
#[path_param]
struct PostId(uuid::Uuid);

#[page("/posts/{post_id}")]
async fn post_page(cx: &Cx) -> Result {
    let post_id = PostId::of(cx).ok_or_redirect("/invalid-id")?;
    view! { "showing post with id: " (post_id.to_string()) }
}
```

## Borrowed `&str` inner type

```ignore
// No parsing — the raw segment value is exposed directly.
#[path_param]
struct Slug<'a>(&'a str);

#[page]
async fn show(cx: &Cx) -> Result {
    let slug = Slug::of(cx); // `&Slug<'_>`
    view! { "slug: " (&**slug) }
}
```

# Requirements

- The item must be a tuple struct with exactly one unnamed field.
- For non-`&str` inner types, the inner type must implement [`FromStr`](core::str::FromStr) and meet the requirements of [`#[memoize]`](macro@memoize) (the parsed `Result` must be `Send + Sync + 'static`).
