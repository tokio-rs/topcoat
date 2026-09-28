Customizes how a module contributes to module-router URLs.

Place `segment!(...)` in a non-root route module to change its segment under [`module_router!`](macro.module_router.html). Regular modules default to their kebab-cased name. Modules starting with `_` default to groups, which add no URL segment. The root module adds no segment. This macro does not affect handlers with absolute paths or a regular [`Router`](struct.Router.html).

# Attributes

The macro takes comma-separated `key = value` attributes, each at most once:

- `rename = "name"`: replaces the segment's name with the literal, used as-is (no kebab-casing).
- `kind = Static`: a literal URL segment; the default for regular modules. Use it to turn a `_`-prefixed module back into a static segment.
- `kind = Group`: no URL segment, though the module can still hold shared layouts and layers; the default for `_`-prefixed modules.
- `kind = Param`: a dynamic `{name}` parameter, matching one segment.
- `kind = CatchAll`: a wildcard `{*name}` tail, matching all remaining segments.

A `Param` or `CatchAll` segment uses the module's exact name unless you set `rename`. [`module_param!`](macro.module_param.html) sets the segment kind and name for you, so do not also use `segment!` in that module. If you use `segment!` alone, read the captured value with [`raw_path_params`](fn.raw_path_params.html). It does not create a type for reading the value with `path_param::<T>(cx)`.

A `CatchAll` matches one or more remaining URL segments, including `/` separators, and must be the last served segment in the path.

For a catch-all, `raw_path_params` provides the rest of the path as an encoded string and as separate decoded segments. To read those segments with `path_param::<T>(cx)`, replace the `segment!` declaration with `module_param!(*name)`.

# Examples

```rust
use topcoat::router::segment;
// src/app/blog_post.rs: module URL becomes `/articles` instead of `/blog-post`.
segment!(rename = "articles");
```

```rust
use topcoat::router::segment;
// src/app/marketing.rs: `marketing` contributes no URL segment.
segment!(kind = Group);
```

```rust
use topcoat::router::segment;
// src/app/_group.rs: `_group` is reachable as `/group`.
segment!(kind = Static);
```

```rust
use topcoat::router::segment;
// src/app/users/id.rs: pages in this module serve `/users/{id}`.
segment!(kind = Param);
```

```rust
use topcoat::router::segment;
// src/app/docs/rest.rs: pages in this module serve `/docs/{*path}`.
segment!(kind = CatchAll, rename = "path");
```
