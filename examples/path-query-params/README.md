# Path and query parameters

This example demonstrates how to read typed path and query parameters from an HTTP request.

It shows how to:

- parse a query string into a typed structure;
- provide default values for optional query parameters;
- extract a dynamic segment from a URL;
- return a custom error when a path parameter cannot be parsed.

## Run the example

From the repository root, run:

```sh
cargo run --manifest-path examples/path-query-params/Cargo.toml
```

The application is served by default at:

```text
http://127.0.0.1:3000
```

Open the address in your browser.

## Available routes

| Route | Description |
| --- | --- |
| `/` | Links to the query and path parameter examples |
| `/posts` | Displays the default query parameter values |
| `/posts?page=2&q=rust` | Parses `page` and `q` from the query string |
| `/posts/42` | Parses `42` from the `{post_id}` path segment |

## Query parameters

Open:

```text
http://127.0.0.1:3000/posts?page=2&q=rust
```

The page should display:

```text
Posts
page: 2
search: rust
```

Both query parameters are optional. Opening `/posts` without a query string displays:

```text
page: 1
search: all
```

An invalid `page` value redirects to the page with the invalid query string cleared.

## Path parameters

Open:

```text
http://127.0.0.1:3000/posts/42
```

The page should display:

```text
Post 42
parsed from the {post_id} path segment
```

The post ID must be a valid unsigned integer.

Opening `/posts/hello` returns a bad request response containing:

```text
Post ID must be a number!
```

## Test the example

Test the query parameters:

```sh
curl --silent \
    "http://127.0.0.1:3000/posts?page=2&q=rust"
```

Test the path parameter:

```sh
curl --silent http://127.0.0.1:3000/posts/42
```

Test an invalid path parameter:

```sh
curl --include http://127.0.0.1:3000/posts/hello
```

Test an invalid query parameter:

```sh
curl --include \
    "http://127.0.0.1:3000/posts?page=invalid"
```

## How it works

- `PostsQuery` defines the accepted query parameters.
- `#[query_params]` enables typed query-string parsing.
- `query_params::<PostsQuery>(cx)` reads the values from the request.
- `Option` allows `page` and `q` to be omitted.
- `PostId` wraps the numeric post identifier.
- `path_param!(post_id: u32, ...)` declares typed path-segment parsing.
- `path_param::<PostId>(cx)` reads `{post_id}` from the request path.
- Invalid post IDs return the custom bad request message.

Stop the server by pressing `Ctrl+C`.
