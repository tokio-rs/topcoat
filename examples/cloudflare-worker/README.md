# Cloudflare Worker

A todo list that runs on Cloudflare Workers and stores its todos in [Cloudflare D1](https://developers.cloudflare.com/d1/) through the Toasty ORM. The Worker's fetch handler passes each request to `Router::handle`, so no server is started.

Clone the Topcoat repository:

```sh
git clone https://github.com/tokio-rs/topcoat.git
cd topcoat/examples/cloudflare-worker
```

Install the `wasm32-unknown-unknown` target and [Wrangler](https://developers.cloudflare.com/workers/wrangler/). Create the `todos` table in a local D1 database, then run the Worker locally:

```sh
rustup target add wasm32-unknown-unknown
npx wrangler d1 migrations apply DB --local
npx wrangler dev
```

To deploy, create a D1 database with `npx wrangler d1 create topcoat-todos` and put its ID in `database_id` in `wrangler.toml`. Then apply the migration with `npx wrangler d1 migrations apply DB --remote` and deploy with `npx wrangler deploy`.

The app leaves out Topcoat's `serve` feature, which needs a native server. Request bodies are read into memory before routing. Runtime WebSocket connections are not available on Workers.

The database handle is built on the first request and shared through the app context. D1 does not support transactions, so see the [Toasty D1 guide](https://github.com/tokio-rs/toasty/blob/main/docs/guide/src/d1.md) for the driver's limits. The schema is managed with [D1 migrations](https://developers.cloudflare.com/d1/reference/migrations/) in `migrations/`, which must match the `Todo` model.
