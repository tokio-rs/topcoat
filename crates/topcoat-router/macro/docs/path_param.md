Declares a typed path parameter.

Name the parameter as it appears in the URL. The macro generates a type for it, named in Pascal case, which you use to read the parameter and to build URLs.

```rust
# use topcoat::router::path_param;
path_param!(post_id: u64);
// Generates `struct PostId(u64)`.
```

# Matching the URL

In an absolute route path, write a `{name}` placeholder with the same name as the declaration.

```rust
# use topcoat::{Result, router::{page, path_param}, view::{View, view}};
path_param!(post_id: u64);

#[page("/posts/{post_id}")]
async fn post() -> Result<impl View> {
    Ok(view! { "post" })
}
```

The declaration also acts as a [`segment!`](macro.segment.html) override. Under [`module_router!`](macro.module_router.html), it turns the segment of the declaring module into the parameter, so pages in that module do not need a path.

```rust
// src/app/posts/id.rs serves /posts/{post_id}.
# use topcoat::{Result, router::{page, path_param}, view::{View, view}};
path_param!(post_id: u64);

#[page]
async fn post() -> Result<impl View> {
    Ok(view! { "post" })
}
```

A module adds one segment, so it can declare only one path parameter. Put further parameters in descendant modules.

Reading a parameter that the matched route did not capture panics with `path parameter "post_id" was not found in request path`.

# Reading one segment

[`path_param::<T>(cx)`](fn.path_param.html) reads the parameter from the matched route. With a type after `:`, the segment is parsed with [`FromStr`](core::str::FromStr). The result is memoized, so the segment is parsed at most once per request.

```rust
# use topcoat::{context::Cx, Result, router::{error::RouterErrorExt, page, path_param}, view::{View, view}};
path_param!(post_id: u64);

#[page("/posts/{post_id}")]
async fn post(cx: &Cx) -> Result<impl View> {
    let post_id: &u64 = path_param::<PostId>(cx).ok_or_not_found()?;
    Ok(view! { "post " (post_id) })
}
```

Without a type, `path_param::<Slug>(cx)` returns the percent-decoded segment as a `&str`. It does not allocate and cannot fail.

```rust
# use topcoat::{context::Cx, Result, router::{page, path_param}, view::{View, view}};
path_param!(slug);

#[page("/posts/{slug}")]
async fn post(cx: &Cx) -> Result<impl View> {
    let slug: &str = path_param::<Slug>(cx);
    Ok(view! { "slug " (slug) })
}
```

A declaration without a type generates `struct Slug<T: AsRef<str> = String>(T)`. Because `String` is the default type argument, you can write just `Slug` in type positions. You can construct it from an owned or a borrowed string.

# Failing with an error response

Add `error = ...` to turn a parse failure into a router error. The handler can then use `?`.

```rust
# use topcoat::{context::Cx, Result, router::{page, path_param}, view::{View, view}};
path_param!(post_id: u64, error = not_found);

#[page("/posts/{post_id}")]
async fn post(cx: &Cx) -> Result<impl View> {
    let post_id = path_param::<PostId>(cx)?;
    Ok(view! { "post " (post_id) })
}
```

The supported forms match the router's error constructors:

- `error = not_found`
- `error = unauthorized`
- `error = forbidden`
- `error = bad_request` or `error = bad_request("description")`
- `error = redirect("/path")`
- `error = redirect_permanent("/path")`

A bare `error = bad_request` uses the description `invalid value for path parameter "post_id"`.

Without `error = ...`, the reader returns `Result<&T, &<T as FromStr>::Err>`, and each call site picks a response, for example with [`RouterErrorExt`](error/trait.RouterErrorExt.html).

A parameter without a type cannot use `error`, because reading it cannot fail.

# Visibility and construction

The visibility you write applies to the generated type and to its field.

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

Keep the declaration private when only the declaring module and its descendants read it. Otherwise use the narrowest visibility that the code naming or constructing the type needs, such as `pub(super)`, `pub(crate)`, or `pub`.

# Catch-all parameters

Put `*` before the name to capture the rest of the path as separate decoded segments. A catch-all must be the last segment of the path, and it matches one or more segments.

```rust
# use topcoat::{context::Cx, Result, router::{CatchAllSegments, page, path_param}, view::{View, view}};
path_param!(*doc_path);

#[page("/docs/{*doc_path}")]
async fn document(cx: &Cx) -> Result<impl View> {
    let path: CatchAllSegments<'_> = path_param::<DocPath>(cx);
    let path = path.collect::<std::path::PathBuf>();
    Ok(view! { (path.display().to_string()) })
}
```

[`CatchAllSegments`](struct.CatchAllSegments.html) yields one decoded `&str` for each URL segment. For `/docs/api%2Frouter/start`, it yields `"api/router"` and then `"start"`. The encoded slash stays inside the first segment.

With a type, a catch-all parses each segment and returns a memoized slice.

```rust
# use topcoat::{context::Cx, Result, router::{page, path_param}, view::{View, view}};
path_param!(*ids: u32, error = bad_request);

#[page("/archive/{*ids}")]
async fn archive(cx: &Cx) -> Result<impl View> {
    let ids: &[u32] = path_param::<Ids>(cx)?;
    Ok(view! { (format!("{ids:?}")) })
}
```

The type after `:` is the type of one segment, so this declaration generates `struct Ids(Vec<u32>)`. Without `error = ...`, the reader returns `Result<&[u32], &<u32 as FromStr>::Err>`. The error is the first parse error, without the index of the segment.

A bare `error = bad_request` includes the zero-based index of the failing segment. For `/archive/1/x/3`, the description is `invalid value for path parameter "ids" at segment 1`.

A catch-all without a type accepts any `IntoIterator` whose items implement `AsRef<str>`. For example, `path_param!(pub *doc_path)` accepts `DocPath(["guide", "start"])`, and `DocPath` in type positions means `DocPath<Vec<String>>`.

# Building URLs

The generated type also works with the [`href!`](macro.href.html) macro, where it fills the matching parameter in a handler's path:

```rust
# use topcoat::{Result, router::{href, page, path_param}, view::{View, view}};
path_param!(post_id: u64);
path_param!(*doc_path);

#[page("/posts/{post_id}")]
async fn post() -> Result<impl View> {
    Ok(view! { "post" })
}

#[page("/docs/{*doc_path}")]
async fn document() -> Result<impl View> {
    Ok(view! { "doc" })
}

#[page("/")]
async fn home() -> Result<impl View> {
    Ok(view! {
        // /posts/1
        <a href=(href!(post, PostId(1)))>"The first post"</a>
        // /docs/guides/getting%20started
        <a href=(href!(document, DocPath(["guides", "getting started"])))>"Guides"</a>
    })
}
```

Values are matched to the path by name, not only by position. Filling `{post_id}` with anything other than a `PostId` panics instead of building a wrong URL.

Each segment is written with [`Display`](core::fmt::Display) and percent-encoded, so a value always stays inside its own segment. For example, `Slug("a/b")` becomes the single segment `a%2Fb`. A catch-all adds one segment per element, so the only `/` it adds are the separators between them.

Filling a segment with an empty string, `.`, or `..` panics. A browser would resolve such a segment against the rest of the path instead of reading it as a segment, even when it is encoded.

[`href`](fn.href.html) takes the same values as a tuple, for building a URL outside a macro.

# Requirements

- A segment type must implement [`FromStr`](core::str::FromStr).
- A segment type must implement [`Display`](core::fmt::Display) to be used with [`href`](fn.href.html).
- The segment type and its `<T as FromStr>::Err` must be `Send + Sync + 'static`, so the result can be [memoized](../context/attr.memoize.html).
- In an absolute route path, the parameter name must match the declaration.
- A module can contain either one `path_param!` or one `segment!`, not both.
