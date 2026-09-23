Declares a catch-all page that resolves every URL it serves to a not-found error.

An unmatched request returns 404 without rendering a layout. Only layers without a path run for it. This macro registers a catch-all page so an outer layout can catch its [`NotFoundError`](error/struct.NotFoundError.html) and show a custom not-found view. See the [error guide](error/index.html) for an example.

With a path, the macro appends a `{*rest}` catch-all segment and expands to a page named `not_found` serving every method under that prefix. Register it like any other explicit-path page: pass `not_found` to [`RouterBuilder::page`](struct.RouterBuilder.html#method.page), or let [`discover`](trait.RouterBuilderDiscoverExt.html) collect it.

Without a path, the macro expands to a `not_found` module holding the catch-all page, deriving the prefix from the enclosing module like any other [`module_router!`](macro.module_router.html) handler.

More specific routes win over the catch-all, which only serves URLs nothing else matches. A catch-all requires at least one segment, so the prefix URL itself (`/` for a root fallback) is not covered and is served by its own page.

# Examples

A site-wide fallback, covering every URL no other route serves:

```rust
use topcoat::router::{Router, not_found};

not_found!("/");

let router = Router::builder().page(not_found).build();
```

A fallback for one subtree only, here `/admin/{*rest}`:

```rust
# use topcoat::router::{Router, not_found};
not_found!("/admin");
# let router = Router::builder().page(not_found).build();
```

Module-derived path (in `src/app/admin.rs` under [`module_router!`](macro.module_router.html), this covers `/admin/{*rest}`):

```rust
topcoat::router::not_found!();
```
