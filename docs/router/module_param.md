`module_param!` is [`path_param!`](macro.path_param.html) for [module routing](macro.module_router.html). It declares the path parameter a module is named after.

A file such as `src/app/posts/post_id.rs` that calls `module_param!(post_id)` serves `/posts/{post_id}`. Its pages read the parameter with `post_id(cx)`, and links pass it as `post_id: 42`.

# Usage

Name the file after the parameter and call `module_param!` at the top. Pages in the module need no path of their own. Read the value with the function the macro generates, which has the parameter's name.

```rust
// src/app/posts/post_id.rs serves /posts/{post_id}.
# use topcoat::{context::Cx, Result, router::{module_param, page}, view::{View, view}};
module_param!(post_id: u64 else bad_request);

#[page]
async fn post(cx: &Cx) -> Result<impl View> {
    let post_id = post_id(cx)?;
    Ok(view! { "post " (post_id) })
}
```

# Linking

Pass `post_id: value` to [`href!`](macro.href.html). This works both inside the module and in its parent, the module that contains the file:

```rust
// src/app/posts.rs
# use topcoat::{context::Cx, Result, router::{href, page}, view::{View, view}};
mod post_id {
    # use topcoat::{context::Cx, Result, router::{module_param, page}, view::{View, view}};
    module_param!(pub post_id: u64 else bad_request);

    #[page]
    pub async fn post(cx: &Cx) -> Result<impl View> {
        Ok(view! { "post" })
    }
}

#[page]
async fn posts(cx: &Cx) -> Result<impl View> {
    // /posts/42
    Ok(view! { <a href=(href!(post_id::post, post_id: 42))>"A post"</a> })
}
# fn main() {}
```

From anywhere else, use the full path, such as `crate::app::posts::post_id: 42`.

# Catch-all parameters

Put `*` before the name to match the rest of the URL path. A catch-all must come last in the route and matches at least one segment.

```rust
// src/app/docs/doc_path.rs serves /docs/{*doc_path}.
# use topcoat::router::module_param;
module_param!(*doc_path);
```

# More options

`module_param!` accepts the same options as [`path_param!`](macro.path_param.html): a type to parse the value into, and an `else` clause that picks the error response when parsing fails.

A module can have only one `module_param!`. For more parameters, add a child module with its own `module_param!`, or use `path_param!` for a parameter in a single handler's path.
