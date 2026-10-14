# Request and response

API handlers that read request bodies and return responses. Includes custom `FromRequest` and `IntoResponse` implementations.

Clone the Topcoat repository:

```sh
git clone https://github.com/tokio-rs/topcoat.git
cd topcoat
```

Then run it with:

```sh
cargo topcoat dev -p request-response
```

The example only serves API routes under `/api`; there is no page at `/`.
