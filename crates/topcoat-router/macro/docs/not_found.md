Declares a catch-all page that answers every URL it serves with a not-found error.

When a request matches no route, the router answers with a plain 404. No layers with a path run and no layout renders around it. This macro declares a catch-all page for those URLs instead. The request is then handled like any other, and the page fails with a [`NotFoundError`](error/struct.NotFoundError.html). An error boundary in an outer layout can catch that error and show a custom not-found view, as described in the [error guide](error/index.html).

With a path, the macro adds a `{*rest}` catch-all segment to it and declares a page named `not_found` that serves every method under that path. Register it like any other page with an absolute path: pass `not_found` to [`RouterBuilder::page`](struct.RouterBuilder.html#method.page), or let [`discover`](trait.RouterBuilderDiscoverExt.html) collect it.

Without a path, the macro declares a `not_found` module that holds the catch-all page. The page gets its path from the enclosing module, like any other handler under [`module_router!`](macro.module_router.html), which also registers it.

Other routes are more specific and always win, so the catch-all only serves URLs that nothing else matches. A catch-all segment matches at least one segment. The path itself (`/` for a site-wide fallback) is therefore not covered and needs its own page.

# Examples

A site-wide fallback that covers every URL no other route serves:

```rust
use topcoat::router::{Router, not_found};

not_found!("/");

let router = Router::builder().page(not_found).build();
```

A fallback for one part of the site, here `/admin/{*rest}`:

```rust
# use topcoat::router::{Router, not_found};
not_found!("/admin");
# let router = Router::builder().page(not_found).build();
```

A module-derived path. In `src/app/admin.rs` under [`module_router!`](macro.module_router.html), this covers `/admin/{*rest}`:

```rust
topcoat::router::not_found!();
```
