Declares a typed path parameter.

Name the parameter as it appears in the URL. The macro generates a function with that name, which reads the parameter from the matched route.

```rust
# use topcoat::router::path_param;
path_param!(post_id: u64);
// Generates `fn post_id(cx: &Cx) -> Result<&u64, &ParseIntError>`.
```

`path_param!` only declares the parameter. You add it to a route by writing `{post_id}` in the handler's path. [`module_param!`](macro.module_param.html) makes the same declaration and also sets the module's URL segment to `{post_id}`.

# Matching the URL

Add the parameter name in braces to the route path, such as `{post_id}`. With [module routing](macro.module_router.html), use a path starting with `./` to add it after the module's path.

```rust
// src/app/posts.rs serves /posts/{post_id}.
# use topcoat::{Result, context::Cx, router::{page, path_param}, view::{View, view}};
path_param!(post_id: u64);

#[page("./{post_id}")]
async fn post(cx: &Cx) -> Result<impl View> {
    Ok(view! { "post" })
}
```

You can declare several parameters in one module and use them in your handler paths. `path_param!` leaves the module's URL segment unchanged.

Reading a parameter that the matched route did not capture panics.

# Reading one segment

Call the generated function, such as `post_id(cx)`, to read the parameter from the matched route. A declaration with `: Type` parses the segment with [`FromStr`](core::str::FromStr) and memoizes the result for the request.

```rust
// src/app/posts.rs
# use topcoat::{context::Cx, Result, router::{error::RouterErrorExt, page, path_param}, view::{View, view}};
path_param!(post_id: u64);

#[page("./{post_id}")]
async fn post(cx: &Cx) -> Result<impl View> {
    let post_id: &u64 = post_id(cx).ok_or_not_found()?;
    Ok(view! { "post " (post_id) })
}
```

Without a type, `slug(cx)` returns the percent-decoded segment as `&str` without allocating or failing.

```rust
// src/app/posts.rs
# use topcoat::{context::Cx, Result, router::{page, path_param}, view::{View, view}};
path_param!(slug);

#[page("./{slug}")]
async fn post(cx: &Cx) -> Result<impl View> {
    let slug: &str = slug(cx);
    Ok(view! { "slug " (slug) })
}
```

# Failing with an error response

An `else` clause after the type maps a parse failure to a router error, so a handler can use `?`.

```rust
// src/app/posts.rs
# use topcoat::{context::Cx, Result, router::{page, path_param}, view::{View, view}};
path_param!(post_id: u64 else not_found);

#[page("./{post_id}")]
async fn post(cx: &Cx) -> Result<impl View> {
    let post_id = post_id(cx)?;
    Ok(view! { "post " (post_id) })
}
```

The supported forms mirror the router's error constructors:

- `else not_found`
- `else unauthorized`
- `else forbidden`
- `else bad_request` or `else bad_request("description")`
- `else redirect("/path")`
- `else redirect_permanent("/path")`

A bare `else bad_request` uses `invalid value for path parameter "post_id"`.

Without an `else` clause, the function returns `Result<&T, &<T as FromStr>::Err>` and the call site chooses a response with [`RouterErrorExt`](error/trait.RouterErrorExt.html).

An unparsed parameter cannot have an `else` clause because it cannot fail.

# Visibility and names

The declared visibility applies to the generated function and to the name [`href!`](macro.href.html) uses to fill the parameter.

```rust
# use topcoat::router::path_param;
path_param!(pub post_id: u64);
```

Keep the declaration private when only the module and its descendants read the parameter or link to it. Use the narrowest visibility needed by other code, such as `pub(super)`, `pub(crate)`, or `pub`.

# Catch-all parameters

Prefix the name with `*` to capture the remaining path as separate decoded segments. A catch-all must be the last served segment and matches at least one segment.

```rust
// src/app/docs.rs
# use topcoat::{context::Cx, Result, router::{CatchAllSegments, page, path_param}, view::{View, view}};
path_param!(*doc_path);

#[page("./{*doc_path}")]
async fn document(cx: &Cx) -> Result<impl View> {
    let path: CatchAllSegments<'_> = doc_path(cx);
    let path = path.collect::<std::path::PathBuf>();
    Ok(view! { (path.display().to_string()) })
}
```

[`CatchAllSegments`](struct.CatchAllSegments.html) yields one decoded `&str` per URL segment. For `/docs/api%2Frouter/start`, it yields `"api/router"` and `"start"`; the encoded slash stays inside the first segment.

A typed catch-all parses each segment and returns a memoized slice.

```rust
// src/app/archive.rs
# use topcoat::{context::Cx, Result, router::{page, path_param}, view::{View, view}};
path_param!(*ids: u32 else bad_request);

#[page("./{*ids}")]
async fn archive(cx: &Cx) -> Result<impl View> {
    let ids: &[u32] = ids(cx)?;
    Ok(view! { (format!("{ids:?}")) })
}
```

The type after `:` is the type of one segment. Without an `else` clause, the function returns `Result<&[u32], &<u32 as FromStr>::Err>`; it returns the first parse error without a segment index.

A bare `else bad_request` includes the zero-based failing segment index. For `/archive/1/x/3`, its description is `invalid value for path parameter "ids" at segment 1`.

# Building URLs

Pass `name: value` to [`href!`](macro.href.html) to fill the matching parameter in a handler's URL. Name a parameter declared in another module by its path, such as `posts::post_id`:

```rust
# use topcoat::{Result, context::Cx, router::{href, page, path_param}, view::{View, view}};
// src/app.rs
mod posts {
    use super::*;

    path_param!(pub post_id: u64);

    #[page("./{post_id}")]
    pub async fn post(cx: &Cx) -> Result<impl View> {
        Ok(view! { "post" })
    }
}

mod docs {
    use super::*;

    path_param!(pub *doc_path);

    #[page("./{*doc_path}")]
    pub async fn document(cx: &Cx) -> Result<impl View> {
        Ok(view! { "doc" })
    }
}

#[page]
async fn home(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        // /posts/1
        <a href=(href!(posts::post, posts::post_id: 1))>"The first post"</a>
        // /docs/guides/getting%20started
        <a href=(href!(docs::document, docs::doc_path: ["guides", "getting started"]))>"Guides"</a>
    })
}
# fn main() {}
```

Each declaration accepts the values you would expect for its type, owned or borrowed:

- `post_id: u64` accepts a `u64` or a reference to one.
- `slug` accepts any string, such as `&str` or `String`.
- `*ids: u32` accepts anything that can be viewed as a slice of `u32`, such as a vector, an array, or a slice.
- `*doc_path` accepts a slice, array, or vector of strings, a reference to one, or the [`CatchAllSegments`](struct.CatchAllSegments.html) read from a request.

Values are borrowed when the URL is written, so the same `href!` can render more than once.

A link that leaves out a parameter of the target's path, or fills a parameter the path does not declare, panics.

Each segment is written with [`Display`](core::fmt::Display) and percent-encoded, so a value stays inside the segment it fills: `slug: "a/b"` fills its one segment as `a%2Fb`. A catch-all contributes one segment per element, so the separators between them are the only `/` it adds.

Empty values, `.`, and `..` panic because browsers treat them as path structure rather than segment content.

# Requirements

- Parsed segment types must implement [`FromStr`](core::str::FromStr) to be read.
- Parsed segment types must implement [`Display`](core::fmt::Display) to be written into a URL.
- The parsed segment type and its `<T as FromStr>::Err` must be `Send + Sync + 'static` so the result can be [memoized](../context/attr.memoize.html).
- The parameter name in the route path must match the declaration.
