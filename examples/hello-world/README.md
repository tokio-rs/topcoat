# Hello world

This example demonstrates the basic structure of a Topcoat application.

It shows how to:

- start a Topcoat HTTP server;
- register a page;
- render a reusable component;
- generate an HTML document with the `view!` macro.

## Run the example

From the repository root, run:

```sh
cargo run --manifest-path examples/hello-world/Cargo.toml
```

Cargo downloads and compiles the required dependencies automatically.

The application is served by default at:

```text
http://127.0.0.1:3000
```

Open that address in your browser.

## Expected result

The browser tab should have the title:

```text
Hello world
```

The page should display:

```text
Hello, World!
```

## Test the example

With the application running, open another terminal and check the HTTP response:

```sh
curl -i http://127.0.0.1:3000/
```

The response should have an HTTP `200` status and contain the rendered `Hello, World!` heading.

You can check only the status code with:

```sh
curl --silent --output /dev/null --write-out "%{http_code}\n" http://127.0.0.1:3000/
```

The expected output is:

```text
200
```

## How it works

- `main` creates the router, discovers the registered routes, and starts the server.
- `#[page("/")]` registers `home` as the page for the root route.
- `view!` renders the HTML document.
- `topcoat::dev::script()` enables browser reload support when using the Topcoat development server.
- `#[component]` turns `hello` into a reusable asynchronous component.
- `hello(name: "World")` renders the `hello` component with `World` as its argument.

## Change the address

Topcoat uses `127.0.0.1:3000` by default.

Set `HOST` or `PORT` to use a different address:

```sh
HOST=0.0.0.0 PORT=8080 cargo run --manifest-path examples/hello-world/Cargo.toml
```

The application will then be available at:

```text
http://127.0.0.1:8080
```

Stop the server by pressing `Ctrl+C`.