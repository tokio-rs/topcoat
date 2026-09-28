Declares a typed path parameter and turns the enclosing module into that parameter.

Under [module routing](macro.module_router.html), `module_param!` changes the module's URL segment to the parameter, so its handlers do not write a path.

```rust
// src/app/posts/post_id.rs serves /posts/{post_id}.
# use topcoat::{context::Cx, Result, router::{module_param, page, path_param}, view::{View, view}};
module_param!(post_id: u64, error = bad_request);

#[page]
async fn post(cx: &Cx) -> Result<impl View> {
    let post_id = path_param::<PostId>(cx)?;
    Ok(view! { "post " (post_id) })
}
```

The macro takes the same input as [`path_param!`](macro.path_param.html) and generates the same type, so read the value with [`path_param::<T>(cx)`](fn.path_param.html) and fill it into an [`href!`](macro.href.html) as usual. See [`path_param!`](macro.path_param.html) for parsing, errors, visibility, and URL building.

The parameter name in the URL comes from the declaration, not from the file name. The file above could be named `id.rs` and would still serve `/posts/{post_id}`.

Prefix the name with `*` to turn the module into a catch-all instead. It must be the last served segment.

```rust
// src/app/docs/doc_path.rs serves /docs/{*doc_path}.
# use topcoat::router::module_param;
module_param!(*doc_path);
```

A module contributes one segment, so it can contain only one `module_param!` or [`segment!`](macro.segment.html). Put another parameter in a descendant module, or declare it with `path_param!` and use it in a relative path such as `#[page("./{comment_id}")]`.

`module_param!` has no effect on handlers with absolute paths or on a regular [`Router`](struct.Router.html).
