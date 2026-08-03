# Error

This example demonstrates how handler errors become HTTP responses and how a layout replaces them with branded error pages.

It serves a small post catalog: a missing post responds 404, an unparsable id 400, the admin area 403, and a catch-all page gives unrouted URLs the same branded 404 as the rest of the application.

## Run the example

From the repository root, run:

```sh
cargo run --manifest-path examples/error/Cargo.toml
```

From inside the `examples` directory, run:

```sh
cargo run --manifest-path error/Cargo.toml
```

The application is served by default at:

```text
http://127.0.0.1:3000
```

## Test in the browser

Open:

```text
http://127.0.0.1:3000
```

The home page links every case: an existing post, a missing post, the admin area, and an unrouted URL. The missing post and the unrouted URL both render the branded "Page not found" view, and the admin area renders "Access denied", each inside the shared layout.

## Test with curl

Check the status code of each route:

```sh
for path in / /posts/1 /posts/7 /posts/oops /admin /no/such/page; do
    curl --silent --output /dev/null --write-out "%{http_code} $path\n" \
        "http://127.0.0.1:3000$path"
done
```

Expected output:

```text
200 /
200 /posts/1
404 /posts/7
400 /posts/oops
403 /admin
404 /no/such/page
```

Fetch an unrouted URL to see the branded body:

```sh
curl --silent http://127.0.0.1:3000/no/such/page
```

The response is the "Page not found" view wrapped in the layout, served with a 404 status.

## How it works

- Every handler returns a `Result`; an `Err` becomes the response, mapped onto its HTTP status code.
- `path_param!(post_id: u64, error = bad_request)` answers an unparsable id with a 400.
- `ok_or_not_found()` replaces the `None` of a missing post with a `NotFoundError`.
- The admin page returns the `forbidden()` constructor directly.
- The root layout downcasts `NotFoundError` and `ForbiddenError` and replaces them with branded views; the `StatusCode` in each view keeps the error's status.
- `not_found!("/")` registers a catch-all page, so a URL matching no route resolves to a `NotFoundError` that flows through the layout instead of the router's bare 404.

Stop the server by pressing `Ctrl+C`.
