# Shard

This example demonstrates how to re-render part of a page on the server when browser state changes.

It shows how to:

- store input text in a browser signal;
- update the signal from an input event;
- pass reactive browser state to a server shard;
- perform a server-side search;
- replace only the shard content without reloading the page.

## Prerequisites

This example uses Topcoat's browser runtime and generated assets.

Install the local Topcoat CLI from the repository root if it is not already installed:

```sh
cargo install --path crates/topcoat-cli --locked
```

## Run the example

From the repository root, enter the example directory:

```sh
cd examples/shard
```

Start the development server:

```sh
topcoat dev
```

The application is served by default at:

```text
http://127.0.0.1:3000
```

## Test in the browser

Open the application in your browser.

Because the initial search text is empty, the page initially displays every fruit in the example data.

Enter:

```text
berry
```

After approximately half a second, the results should contain:

```text
blackberry
blueberry
cranberry
elderberry
raspberry
strawberry
```

Replace the search text with:

```text
mango
```

The results should contain only:

```text
mango
```

Enter a value that does not match any fruit:

```text
zzzz
```

The results list should become empty.

The page should not perform a complete reload while the results change.

## Test case-insensitive search

Enter:

```text
BERRY
```

The same berry results should appear because the server converts the search text to lowercase.

## Test the page route

With the application running, send a request from another terminal:

```sh
curl --include http://127.0.0.1:3000/
```

The response should have an HTTP `200` status and contain:

```text
results:
```

## How it works

- `signal input` stores the current search text in the browser.
- `:value` keeps the input element synchronized with the signal.
- `@input` updates the signal after every input event.
- `combobox_content(input: $(input.get()))` passes reactive state to the shard.
- `#[shard]` exposes a server-rendered component that can be refreshed from the browser.
- `search_fruit` performs the lookup on the server.
- The artificial delay simulates a database or external service request.
- Only the shard HTML is replaced when the result changes.
- `AssetBundle::load()` loads the runtime assets.
- `.discover()` registers the page and the endpoint used by the shard.

Shard arguments originate from the browser and must be validated before being used in a real application.

Stop the development server by pressing `Ctrl+C`.