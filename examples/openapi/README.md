# OpenAPI

This example documents a small JSON API and serves its OpenAPI document. A layer attaches an `Operation` to every route under `/api`, and each route adds its own summary, parameters, request body, and responses.

Clone the repository and open its directory:

```sh
git clone https://github.com/tokio-rs/topcoat.git
cd topcoat
```

Start the example:

```sh
cargo topcoat dev -p openapi
```

Open `/openapi.json` to see the generated document. The API routes expect the header `Authorization: Bearer topcoat-demo`.
