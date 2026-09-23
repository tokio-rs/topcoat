Changes the path segment that a module adds under [`module_router!`](macro.module_router.html).

By default, each module below the root adds its name in kebab-case as a segment, and a module whose name starts with `_` is a group that adds no segment. Call `segment!(...)` in a module to change this. The module that calls `module_router!` is the root and adds no segment, so `segment!` has no effect there. It also has no effect on a [`Router`](struct.Router.html) built without `module_router!`, or on handlers with an absolute path.

# Attributes

The macro takes comma-separated `key = value` attributes. Each can appear at most once:

- `rename = "name"`: uses the given name for the segment exactly as written, without converting it to kebab-case.
- `kind = Static`: a literal URL segment. This is the default for regular modules. Use it to turn a `_`-prefixed module back into a normal segment.
- `kind = Group`: adds no URL segment, but the module can still hold layouts and layers for its descendants. This is the default for `_`-prefixed modules.
- `kind = Param`: a dynamic `{name}` parameter that matches one segment.
- `kind = CatchAll`: a `{*name}` catch-all that matches all remaining segments.

A `Param` or `CatchAll` segment without a `rename` uses the module name as written. [`path_param!`](macro.path_param.html) declares the matching segment for its module, so do not also call `segment!` in that module. A `Param` or `CatchAll` declared with `segment!` captures the segment but does not generate a type for reading it. Read it with [`raw_path_params`](fn.raw_path_params.html) instead.

A `CatchAll` matches one or more segments, including the `/` between them. It must be the last segment of the path. `raw_path_params` gives both the encoded rest of the path and each segment decoded on its own. To read the segments through a generated type, use `path_param!(*name)` instead.

# Examples

```rust
// src/app/blog_post.rs: the module is served at `/articles` instead of `/blog-post`.
topcoat::router::segment!(rename = "articles");
```

```rust
// src/app/marketing.rs: `marketing` adds no URL segment.
topcoat::router::segment!(kind = Group);
```

```rust
// src/app/_group.rs: `_group` is served at `/group`.
topcoat::router::segment!(kind = Static);
```

```rust
// src/app/users/id.rs: pages in this module serve `/users/{id}`.
topcoat::router::segment!(kind = Param);
```

```rust
// src/app/docs/rest.rs: pages in this module serve `/docs/{*path}`.
topcoat::router::segment!(kind = CatchAll, rename = "path");
```
