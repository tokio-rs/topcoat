# Admin Panel

Backroom is a demo admin app for an online store. Staff can sign in, update orders, edit products, and view customer histories. The app uses Toasty with SQLite, session cookies for sign-in, and Topcoat UI components.

Clone the repository and start the app:

```sh
git clone https://github.com/tokio-rs/topcoat.git
cd topcoat
cargo topcoat dev -p admin
```

On startup, the app applies database migrations and adds demo data if needed. The database is stored in `demos/admin/data.db`. The sign-in page shows the demo accounts and password. Stop the app and delete the database to reset it.

The dashboard's sales and visitor counts are simulated from the current time. They do not come from the orders in the database.

## Where to look

| Path | Contents |
| --- | --- |
| [`src/app.rs`](src/app.rs) | Router setup and the shared HTML layout. |
| [`src/app/login.rs`](src/app/login.rs) | Sign-in form and request handler. |
| [`src/app/_admin.rs`](src/app/_admin.rs) | Sign-in check, sidebar, and breadcrumbs. |
| [`src/app/_admin/_dashboard.rs`](src/app/_admin/_dashboard.rs) | Updating dashboard cards and orders awaiting action. |
| [`src/app/_admin/orders.rs`](src/app/_admin/orders.rs) | Order search, filters, and pagination. |
| [`src/app/_admin/orders/order_id.rs`](src/app/_admin/orders/order_id.rs) | Order details and status changes. |
| [`src/app/_admin/products.rs`](src/app/_admin/products.rs) | Product search, prices, and stock levels. |
| [`src/app/_admin/products/product_id.rs`](src/app/_admin/products/product_id.rs) | Product editing and form errors. |
| [`src/app/_admin/customers.rs`](src/app/_admin/customers.rs) | Customer search and spending totals. |
| [`src/app/_admin/customers/customer_id.rs`](src/app/_admin/customers/customer_id.rs) | Customer details and past orders. |
| [`src/features/`](src/features) | Database models, queries, validation, and simulated statistics. |
| [`src/seed.rs`](src/seed.rs) | Sample accounts, products, customers, and orders. |
| [`src/components/`](src/components) | Copies of registry components, plus custom charts and pagination. |
| [`toasty/`](toasty) | Database migrations and schema snapshots. |

## Changing a model

When you change a model, run this from the repository root to generate a migration:

```sh
cargo run -p admin -- toasty migration generate --name <name>
```

Rebuild and restart the app to apply the new migration. Migrations are included in the compiled app.

## Updating components

Run `topcoat ui add <name> --overwrite` from `demos/admin` to update a component from the registry. `cargo test -p admin` checks that the installed components and theme match the registry. Custom components are excluded from this check through `OWN_COMPONENTS` in [`tests/registry_sync.rs`](tests/registry_sync.rs).
