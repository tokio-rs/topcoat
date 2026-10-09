# Coffee shop

A storefront with a searchable menu and drink ordering. It shows how to combine server-rendered pages with browser interactions.

Run it from a checkout of this repository:

```sh
cargo topcoat dev -p coffee-shop
```

The demo has to stay inside the workspace. Its manifest sets `version.workspace` and `edition.workspace`, and every dependency is a `{ workspace = true }` entry, so `cargo` looks for the workspace root to read them from. Copying `demos/coffee-shop` out of the checkout therefore fails to parse:

```
error inheriting `edition` from workspace root manifest's `workspace.package.edition`
Caused by: failed to find a workspace root
```

Clone the repository and run the command from its root instead.

Start with [`src/app.rs`](src/app.rs) for the app's routing and layout. The menu is in [`src/app/menu.rs`](src/app/menu.rs), and drink selection and ordering are in [`src/app/menu/drink.rs`](src/app/menu/drink.rs).
