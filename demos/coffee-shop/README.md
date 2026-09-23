# Coffee shop

A small coffee shop storefront that shows many Topcoat features working together: routes derived from the module tree, vendored `topcoat ui` components, a menu that is loaded once per request and shared, a returning customer read from a cookie by a plain function, and a menu that you can search and order from without a page reload.

Run it from the repository root with:

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
| `suspense` and `error_boundary`: the menu streams in behind a skeleton | [`src/app/menu.rs`](src/app/menu.rs) |
| `path_param!`, runtime expressions, a `#[procedure]` | [`src/app/menu/drink.rs`](src/app/menu/drink.rs) |
| Tailwind, a Fontsource font, an `asset!` image | [`build.rs`](build.rs), [`src/app.rs`](src/app.rs) |
