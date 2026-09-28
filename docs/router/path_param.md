Declares a typed path parameter.

Name the parameter as it appears in the URL. The macro converts that name to Pascal case for the generated type.

```rust
# use topcoat::router::path_param;
path_param!(post_id: u64);
// Generates `struct PostId(u64)`.
```

# Matching the URL

Add the parameter name in braces to the route path, such as `{post_id}`. With [module routing](macro.module_router.html), use a path starting with `./` to add it after the module's path.

```rust
// src/app/posts.rs serves /posts/{post_id}.
# use topcoat::{Result, router::{page, path_param}, view::{View, view}};
path_param!(post_id: u64);

#[page("./{post_id}")]
async fn post() -> Result<impl View> {
    Ok(view! { "post" })
}
```

You can declare several parameters in one module and use them in your handler paths. `path_param!` leaves the module's URL segment unchanged.

```rust
// src/app/posts.rs serves /posts/{post_id}/comments/{comment_id}.
# use topcoat::{Result, router::{page, path_param}, view::{View, view}};
path_param!(post_id: u64);
path_param!(comment_id: u64);

#[page("./{post_id}/comments/{comment_id}")]
async fn comment() -> Result<impl View> {
    Ok(view! { "comment" })
}
```

Use [`module_param!`](macro.module_param.html) when the parameter should be the module's URL segment. It accepts the same options and creates the same type as `path_param!`.

Reading a parameter that the matched route did not capture panics.

# Reading one segment

[`path_param::<T>(cx)`](fn.path_param.html) reads the parameter from the matched route. A declaration with `: Type` parses the segment with [`FromStr`](core::str::FromStr) and memoizes the result for the request.

```rust
// src/app/posts.rs
# use topcoat::{context::Cx, Result, router::{error::RouterErrorExt, page, path_param}, view::{View, view}};
path_param!(post_id: u64);

#[page("./{post_id}")]
async fn post(cx: &Cx) -> Result<impl View> {
    let post_id: &u64 = path_param::<PostId>(cx).ok_or_not_found()?;
    Ok(view! { "post " (post_id) })
}
```

Without a type, `path_param::<Slug>(cx)` returns the percent-decoded segment as `&str` without allocating or failing.

```rust
// src/app/posts.rs
# use topcoat::{context::Cx, Result, router::{page, path_param}, view::{View, view}};
path_param!(slug);

#[page("./{slug}")]
async fn post(cx: &Cx) -> Result<impl View> {
    let slug: &str = path_param::<Slug>(cx);
    Ok(view! { "slug " (slug) })
}
```

The unparsed declaration generates `struct Slug<T: AsRef<str> = String>(T)`. `String` is the default type argument, so type positions can use `Slug`; construction accepts owned or borrowed strings.

# Failing with an error response

`error = ...` maps a parse failure to a router error, so a handler can use `?`.

```rust
// src/app/posts.rs
# use topcoat::{context::Cx, Result, router::{page, path_param}, view::{View, view}};
path_param!(post_id: u64, error = not_found);

#[page("./{post_id}")]
async fn post(cx: &Cx) -> Result<impl View> {
    let post_id = path_param::<PostId>(cx)?;
    Ok(view! { "post " (post_id) })
}
```

The supported forms mirror the router's error constructors:

- `error = not_found`
- `error = unauthorized`
- `error = forbidden`
- `error = bad_request` or `error = bad_request("description")`
- `error = redirect("/path")`
- `error = redirect_permanent("/path")`

A bare `error = bad_request` uses `invalid value for path parameter "post_id"`.

Without `error = ...`, the reader returns `Result<&T, &<T as FromStr>::Err>` and the call site chooses a response with [`RouterErrorExt`](error/trait.RouterErrorExt.html).

An unparsed parameter cannot use `error` because it cannot fail.

# Visibility and construction

The declared visibility applies to the generated type and its field.

```rust
# use topcoat::router::path_param;
path_param!(pub post_id: u64);
path_param!(pub slug);
path_param!(pub *ids: u32);

let id = PostId(42);
let borrowed = Slug("first-post");
let owned: Slug = Slug("first-post".to_owned());
let ids = Ids(vec![1, 2, 3]);
# let _ = (id, borrowed, owned, ids);
```

Keep the declaration private when only descendant modules read it. Use the narrowest visibility needed by code that names or constructs the type, such as `pub(super)`, `pub(crate)`, or `pub`.

# Catch-all parameters

Prefix the name with `*` to capture the remaining path as separate decoded segments. A catch-all must be the last served segment and matches at least one segment.

```rust
// src/app/docs.rs
# use topcoat::{context::Cx, Result, router::{CatchAllSegments, page, path_param}, view::{View, view}};
path_param!(*doc_path);

#[page("./{*doc_path}")]
async fn document(cx: &Cx) -> Result<impl View> {
    let path: CatchAllSegments<'_> = path_param::<DocPath>(cx);
    let path = path.collect::<std::path::PathBuf>();
    Ok(view! { (path.display().to_string()) })
}
```

[`CatchAllSegments`](struct.CatchAllSegments.html) yields one decoded `&str` per URL segment. For `/docs/api%2Frouter/start`, it yields `"api/router"` and `"start"`; the encoded slash stays inside the first segment.

A typed catch-all parses each segment and returns a memoized slice.

```rust
// src/app/archive.rs
# use topcoat::{context::Cx, Result, router::{page, path_param}, view::{View, view}};
path_param!(*ids: u32, error = bad_request);

#[page("./{*ids}")]
async fn archive(cx: &Cx) -> Result<impl View> {
    let ids: &[u32] = path_param::<Ids>(cx)?;
    Ok(view! { (format!("{ids:?}")) })
}
```

The type after `:` is the type of one segment, so this declaration generates `struct Ids(Vec<u32>)`. Without `error = ...`, the reader returns `Result<&[u32], &<u32 as FromStr>::Err>`; it returns the first parse error without a segment index.

A bare `error = bad_request` includes the zero-based failing segment index. For `/archive/1/x/3`, its description is `invalid value for path parameter "ids" at segment 1`.

An unparsed catch-all accepts any `IntoIterator` whose items implement `AsRef<str>`. For example, `path_param!(pub *doc_path)` accepts `DocPath(["guide", "start"])` and defaults to `DocPath<Vec<String>>` in type positions.

# Building URLs

Pass the generated type to [`href!`](macro.href.html) to fill the matching parameter in a handler's URL:

```rust
# use topcoat::{Result, router::{href, page, path_param}, view::{View, view}};
// src/app.rs
mod posts {
    use super::*;

    path_param!(pub post_id: u64);

    #[page("./{post_id}")]
    pub async fn post() -> Result<impl View> {
        Ok(view! { "post" })
    }
}

mod docs {
    use super::*;

    path_param!(pub *doc_path);

    #[page("./{*doc_path}")]
    pub async fn document() -> Result<impl View> {
        Ok(view! { "doc" })
    }
}

#[page]
async fn home() -> Result<impl View> {
    Ok(view! {
        // /posts/1
        <a href=(href!(posts::post, posts::PostId(1)))>"The first post"</a>
        // /docs/guides/getting%20started
        <a href=(href!(docs::document, docs::DocPath(["guides", "getting started"])))>"Guides"</a>
    })
}
# fn main() {}
```

Each argument must have the name expected by its path parameter. For example, filling `{post_id}` with a parameter named `slug` panics.

Each segment is written with [`Display`](core::fmt::Display) and percent-encoded, so a value stays inside the segment it fills: `Slug("a/b")` fills its one segment as `a%2Fb`. A catch-all contributes one segment per element, so the separators between them are the only `/` it adds.

Empty values, `.`, and `..` panic because browsers treat them as path structure rather than segment content.

[`href`](fn.href.html) takes the same values as a tuple, for a URL built outside a macro.

# Requirements

- Parsed segment types must implement [`FromStr`](core::str::FromStr).
- Parsed segment types must implement [`Display`](core::fmt::Display) to be filled into an [`href`](fn.href.html).
- The parsed segment type and its `<T as FromStr>::Err` must be `Send + Sync + 'static` so the result can be [memoized](../context/attr.memoize.html).
- The parameter name in the route path must match the declaration.
