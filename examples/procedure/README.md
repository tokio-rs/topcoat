# Procedure

This example demonstrates how to call an asynchronous Rust function on the server from a browser event handler.

It shows how to:

- manage browser state with a signal;
- read the value of an input element;
- call a server procedure from the browser;
- use the procedure response to update the page.

## Prerequisites

This example uses Topcoat's browser runtime, which is served as a bundled asset.

Install the Topcoat CLI from the repository root:

```sh
cargo install --path crates/topcoat-cli --locked
```

This installation step is required only once.

## Run the example

Enter the example directory:

```sh
cd examples/procedure
```

Start the Topcoat development server:

```sh
cargo topcoat dev
```

The development server builds the application, bundles its assets, and serves it at:

```text
http://127.0.0.1:3000
```

Open that address in your browser.

## Test the example

Enter a message in the input:

```text
hello topcoat
```

Press `Tab` or click outside the input so that its `change` event fires.

Click **Print on server**.

The server terminal should print:

```text
hello topcoat
```

The input in the browser should change to:

```text
message received: hello topcoat
```

## How it works

- `AssetBundle::load()` loads the generated browser assets.
- `topcoat::runtime::script()` loads the Topcoat browser runtime.
- `signal input` stores the current input value in the browser.
- `:value` keeps the input synchronized with the signal.
- `@change` updates the signal when the input value changes.
- `@click` runs an asynchronous browser event handler.
- `print_on_server(input.get()).await` calls the Rust procedure.
- `#[procedure]` exposes the asynchronous Rust function to the browser runtime.
- The procedure prints the submitted value and returns a new string.
- `input.set(server_response)` displays the response in the input.

Procedure arguments originate from the client and must be validated before they are used in a real application.

Stop the development server by pressing `Ctrl+C`.