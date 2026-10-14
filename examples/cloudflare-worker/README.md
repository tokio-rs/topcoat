# Cloudflare Worker

An app that runs on Cloudflare Workers. The Worker's fetch handler passes each request to `Router::handle`, so no server is started.

Clone the Topcoat repository:

```sh
git clone https://github.com/tokio-rs/topcoat.git
cd topcoat/examples/cloudflare-worker
```

Install the `wasm32-unknown-unknown` target and [Wrangler](https://developers.cloudflare.com/workers/wrangler/), then run it locally with:

```sh
rustup target add wasm32-unknown-unknown
npx wrangler dev
```

Deploy it to your Cloudflare account with `npx wrangler deploy`.

The app leaves out Topcoat's `serve` feature, which needs a native server. Request bodies are read into memory before routing. Runtime WebSocket connections are not available on Workers.
