Uses a path parameter as the module's URL segment.

With [module routing](macro.module_router.html), `module_param!(post_id: u64)` gives the module a `{post_id}` segment. A page in that module can use `#[page]` without a path.

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

Read the value with [`path_param::<PostId>(cx)`](fn.path_param.html). To build a link, pass a value such as `PostId(42)` to [`href!`](macro.href.html).

`module_param!` accepts the same options and creates the same type as [`path_param!`](macro.path_param.html). See that guide for parsing, error handling, visibility, and building URLs.

The name in `module_param!` sets the parameter name in the URL. In this example, naming the file `id.rs` would still give the page the path `/posts/{post_id}`.

Put `*` before the name to match the rest of the URL path. This catch-all must come last in the route and match at least one segment.

```rust
// src/app/docs/doc_path.rs serves /docs/{*doc_path}.
# use topcoat::router::module_param;
module_param!(*doc_path);
```

A module adds one URL segment. It can contain one `module_param!` or one [`segment!`](macro.segment.html), but not both. To add another parameter, use `module_param!` in a child module, or use `path_param!` with a relative path such as `#[page("./{comment_id}")]`.

The module's segment only applies to module routing. It does not change absolute handler paths or paths registered with a regular [`Router`](struct.Router.html).
