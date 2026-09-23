# Coffee shop

A storefront with a searchable menu and drink ordering. It shows how to combine server-rendered pages with browser interactions.

Run it with:

```sh
cargo topcoat dev -p coffee-shop
```

Start with [`src/app.rs`](src/app.rs) for the app's routing and layout. The menu is in [`src/app/menu.rs`](src/app/menu.rs), and drink selection and ordering are in [`src/app/menu/drink.rs`](src/app/menu/drink.rs).
