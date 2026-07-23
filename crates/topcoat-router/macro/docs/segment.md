Customizes how a module contributes to module-router URLs.

`segment!(...)` is placed at the top of a non-root route module to override the URL segment the module contributes to a [`module_router!`](macro.module_router.html): by default the kebab-cased module name, or no segment for `_`-prefixed modules (groups). The module containing `module_router!` is the route root and contributes no segment. `segment!` has no effect on a regular [`Router`](struct.Router.html), nor on items whose attribute carries an explicit path.

# Attributes

The macro takes comma-separated `key = value` attributes, each at most once:

- `rename = "name"`: replaces the segment's name with the literal, used as-is (no kebab-casing).
- `kind = Static`: a literal URL segment; the default for regular modules. Use it to turn a `_`-prefixed module back into a static segment.
- `kind = Group`: no URL segment, though the module can still hold shared layouts and layers; the default for `_`-prefixed modules.
- `kind = Param`: a dynamic `{name}` parameter, matching one segment.
- `kind = CatchAll`: a wildcard `{*name}` tail, matching all remaining segments.

A `Param` or `CatchAll` segment without a `rename` is named after the module, as-is. Declaring a [`#[path_param]`](attr.path_param.html) in a module emits `segment!(kind = Param, rename = ...)` automatically, so do not also call `segment!` in that module. A manual `Param` or `CatchAll` declaration creates a captured segment but no typed accessor; read it through [`raw_path_params`](fn.raw_path_params.html).

A `CatchAll` matches one or more remaining URL segments, including `/` separators, and must be the last served segment in the path.

# Examples

```rust
// src/app/blog_post.rs: module URL becomes `/articles` instead of `/blog-post`.
topcoat::router::segment!(rename = "articles");
```

```rust
// src/app/marketing.rs: `marketing` contributes no URL segment.
topcoat::router::segment!(kind = Group);
```

```rust
// src/app/_group.rs: `_group` is reachable as `/group`.
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
