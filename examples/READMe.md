# Examples

This directory contains runnable applications demonstrating the main Topcoat features.

Each example includes its own README with:

- an overview of the demonstrated feature;
- instructions for running the application;
- the expected behavior;
- browser or command-line testing steps;
- an explanation of the relevant code.

Most examples are served at:

```text
http://127.0.0.1:3000
```

## Running an example

Simple examples can usually be run from the repository root with:

```sh
cargo run --manifest-path examples/<example>/Cargo.toml
```

For example:

```sh
cargo run --manifest-path examples/hello-world/Cargo.toml
```

Examples that use generated assets, the browser runtime, Tailwind CSS, fonts, or bundled JavaScript should be run with the Topcoat development server.

Install the local Topcoat CLI once from the repository root:

```sh
cargo install --path crates/topcoat-cli --locked
```

Then enter the example directory and start the development server:

```sh
cd examples/<example>
topcoat dev
```

Check the README inside each example before running it.

## Examples

### Fundamentals

- [`hello-world`](hello-world): create a page and a reusable component.
- [`context`](context): read request information through `Cx`.
- [`app-context`](app-context): share application state across requests.
- [`request-response`](request-response): work with request extractors and response types.
- [`path-query-params`](path-query-params): parse typed path and query parameters.

### Routing

- [`manual-router`](manual-router): register layouts, pages, and routes manually.
- [`module-router`](module-router): derive routes from the Rust module tree.

### Browser interactivity

- [`runtime`](runtime): use signals, event handlers, and reactive expressions.
- [`procedure`](procedure): call an asynchronous Rust function from the browser.
- [`shard`](shard): re-render part of a page on the server.
- [`alpine-ajax`](alpine-ajax): update page fragments with Alpine AJAX.
- [`htmx`](htmx): update page fragments with htmx.
- [`datastar`](datastar): patch signals and elements with Datastar.
- [`sse`](sse): stream Server-Sent Events to the browser.
- [`websocket`](websocket): exchange messages over a WebSocket connection.

### State

- [`cookie`](cookie): store typed data in a signed cookie.
- [`session`](session): implement a session-based login and logout flow.

### Assets and styling

- [`asset`](asset): bundle and serve a local asset.
- [`font`](font): self-host and load web fonts.
- [`icon`](icon): render inline SVG and Iconify icons.
- [`tailwind`](tailwind): generate and serve Tailwind CSS.
- [`ui`](ui): build a component library with Topcoat UI.

### Applications and integrations

- [`mail`](mail): create multipart email messages with a file transport.
- [`toasty-todo`](toasty-todo): build a CRUD application with Toasty and SQLite.

## Verification

Before opening a pull request, verify every example using the testing instructions in its README.

Check that every example directory contains a README:

```sh
for dir in examples/*/; do
    if [ ! -f "${dir}README.md" ]; then
        echo "Missing README: $dir"
    fi
done
```

The command should not print anything.

Count the example README files:

```sh
find examples \
    -mindepth 2 \
    -maxdepth 2 \
    -name README.md \
    | wc -l
```

The expected result is:

```text
24
```

For complete repository verification, follow the instructions in:

```text
.agents/skills/check/SKILL.md
```