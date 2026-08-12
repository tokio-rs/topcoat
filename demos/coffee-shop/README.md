# Coffee shop

A small storefront that tours Topcoat's features together: routes derived from the module tree, vendored `topcoat ui` components, a memoized menu shared across the request, a customer read from a cookie by a plain function, and a menu that searches and orders without a page reload.

Run it with:

```sh
cargo topcoat dev -p coffee-shop
```

Each feature lives in one small file:

| Feature | File |
|---|---|
| `module_router!`, layouts, a POST route | [`src/app.rs`](src/app.rs) |
| `#[memoize]`: the menu loads once per request | [`src/models/drink.rs`](src/models/drink.rs) |
| Functions, not middlewares: the customer cookie | [`src/customer.rs`](src/customer.rs) |
| `topcoat ui` components, vendored source | [`src/components/`](src/components) |
| `view!` control flow and `#[component]` props | [`src/app/menu.rs`](src/app/menu.rs) |
| Signals and a `#[shard]`: live menu search | [`src/app/menu.rs`](src/app/menu.rs) |
| `path_param!`, runtime expressions, a `#[procedure]` | [`src/app/menu/drink.rs`](src/app/menu/drink.rs) |
| Tailwind, a Fontsource font, an `asset!` image | [`build.rs`](build.rs), [`src/app.rs`](src/app.rs) |
