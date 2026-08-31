Declares a typed path parameter.

Name the parameter as it appears in the URL. The macro converts that name to Pascal case for the generated type.

```rust
# use topcoat::router::path_param;
path_param!(post_id: u64);
// Generates `struct PostId(u64)`.
```

# Matching the URL

For an explicit route path, write a placeholder with the declaration's name.

```rust
# use topcoat::{Result, router::{page, path_param}, view::{View, view}};
path_param!(post_id: u64);

#[page("/posts/{post_id}")]
async fn post() -> Result<impl View> {
    Ok(view! { "post" })
}
```

The declaration also emits a [`segment!`](macro.segment.html) override. Under [`module_router!`](../router/macro.module_router.html), it changes the declaring module's segment to the parameter, so the page does not write a path.

```rust
// src/app/posts/id.rs serves /posts/{post_id}.
# use topcoat::{Result, router::{page, path_param}, view::{View, view}};
path_param!(post_id: u64);

#[page]
async fn post() -> Result<impl View> {
    Ok(view! { "post" })
}
```

A module contributes one segment and can declare one path parameter. Put another parameter in a descendant module.

Reading a name that the matched route did not capture panics with `path parameter "post_id" was not found in request path`.

# Reading one segment

[`path_param::<T>(cx)`](fn.path_param.html) reads the parameter from the matched route. A declaration with `: Type` parses the segment with [`FromStr`](core::str::FromStr) and memoizes the result for the request.

```rust
# use topcoat::{context::Cx, Result, router::{error::RouterErrorExt, page, path_param}, view::{View, view}};
path_param!(post_id: u64);

#[page("/posts/{post_id}")]
async fn post(cx: &Cx) -> Result<impl View> {
    let post_id: &u64 = path_param::<PostId>(cx).ok_or_not_found()?;
    Ok(view! { "post " (post_id) })
}
```

Without a type, `path_param::<Slug>(cx)` returns the percent-decoded segment as `&str` without allocating or failing.

```rust
# use topcoat::{context::Cx, Result, router::{page, path_param}, view::{View, view}};
path_param!(slug);

#[page("/posts/{slug}")]
async fn post(cx: &Cx) -> Result<impl View> {
    let slug: &str = path_param::<Slug>(cx);
    Ok(view! { "slug " (slug) })
}
```

The unparsed declaration generates `struct Slug<T: AsRef<str> = String>(T)`. `String` is the default type argument, so type positions can use `Slug`; construction accepts owned or borrowed strings.

# Failing with an error response

`error = ...` maps a parse failure to a router error, so a handler can use `?`.

```rust
# use topcoat::{context::Cx, Result, router::{page, path_param}, view::{View, view}};
path_param!(post_id: u64, error = not_found);

#[page("/posts/{post_id}")]
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
# use topcoat::{context::Cx, Result, router::{CatchAllSegments, page, path_param}, view::{View, view}};
path_param!(*doc_path);

#[page("/docs/{*doc_path}")]
async fn document(cx: &Cx) -> Result<impl View> {
    let path: CatchAllSegments<'_> = path_param::<DocPath>(cx);
    let path = path.collect::<std::path::PathBuf>();
    Ok(view! { (path.display().to_string()) })
}
```

[`CatchAllSegments`](struct.CatchAllSegments.html) yields one decoded `&str` per URL segment. For `/docs/api%2Frouter/start`, it yields `"api/router"` and `"start"`; the encoded slash stays inside the first segment.

A typed catch-all parses each segment and returns a memoized slice.

```rust
# use topcoat::{context::Cx, Result, router::{page, path_param}, view::{View, view}};
path_param!(*ids: u32, error = bad_request);

#[page("/archive/{*ids}")]
async fn archive(cx: &Cx) -> Result<impl View> {
    let ids: &[u32] = path_param::<Ids>(cx)?;
    Ok(view! { (format!("{ids:?}")) })
}
```

The type after `:` is the type of one segment, so this declaration generates `struct Ids(Vec<u32>)`. Without `error = ...`, the reader returns `Result<&[u32], &<u32 as FromStr>::Err>`; it returns the first parse error without a segment index.

A bare `error = bad_request` includes the zero-based failing segment index. For `/archive/1/x/3`, its description is `invalid value for path parameter "ids" at segment 1`.

An unparsed catch-all accepts any `IntoIterator` whose items implement `AsRef<str>`. For example, `path_param!(pub *doc_path)` accepts `DocPath(["guide", "start"])` and defaults to `DocPath<Vec<String>>` in type positions.

# Building URLs

The declared type can also be used in combination with the [`href!`](macro.href.html) macro. It fills the parameter slot of the handler's path to construct a URL string:

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

Values are matched to the path by name, not by position alone, so filling `{post_id}` with anything but a `PostId` panics rather than building a wrong URL.

Each segment is written with [`Display`](core::fmt::Display) and percent-encoded, so a value stays inside the segment it fills: `Slug("a/b")` fills its one segment as `a%2Fb`. A catch-all contributes one segment per element, so the separators between them are the only `/` it adds.

Filling a segment with nothing, `.`, or `..` panics. A browser resolves those against the path around them instead of reading them as one segment, and encoding them does not take that meaning away.

[`href`](fn.href.html) takes the same values as a tuple, for a URL built outside a macro.

# Requirements

- Parsed segment types must implement [`FromStr`](core::str::FromStr).
- Parsed segment types must implement [`Display`](core::fmt::Display) to be filled into an [`href`](fn.href.html).
- The parsed segment type and its `<T as FromStr>::Err` must be `Send + Sync + 'static` so the result can be [memoized](../topcoat_core_macro/attr.memoize.html).
- The parameter name in an explicit route must match the declaration.
- A module can contain either one `path_param!` declaration or one manual `segment!` override.
