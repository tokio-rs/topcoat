# WebSocket

This example demonstrates how to create a WebSocket endpoint with Topcoat.

It shows how to:

- upgrade an HTTP request to a WebSocket connection;
- receive messages from a browser;
- send messages back through the same connection;
- serve the browser code as a bundled asset;
- handle the WebSocket connection asynchronously.

## Prerequisites

This example uses a bundled JavaScript asset.

Install the local Topcoat CLI from the repository root if it is not already installed:

```sh
cargo install --path crates/topcoat-cli --locked
```

## Run the example

From the repository root, enter the example directory:

```sh
cd examples/websocket
```

Start the Topcoat development server:

```sh
topcoat dev
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

The log should display:

```text
connected
```

Enter a message such as:

```text
hello topcoat
```

Click **Send**.

The log should display:

```text
sent: hello topcoat
received: hello topcoat
```

The second line confirms that the server received the message and sent it back through the WebSocket connection.

Send several different messages. Every sent message should be followed by the same received message without reloading the page.

## Inspect the connection

Open the browser developer tools and select the **Network** panel.

Filter the requests by **WS** and select the connection to:

```text
/echo
```

The connection should have an HTTP `101 Switching Protocols` handshake.

The **Messages** panel should show the messages sent to and received from the server.

## Test the page route

From another terminal, run:

```sh
curl --include http://127.0.0.1:3000/
```

The response should have an HTTP `200` status and contain:

```text
WebSocket echo
```

A normal `curl` request cannot test the complete WebSocket exchange because it does not perform and maintain the WebSocket protocol connection.

## How it works

- `AssetBundle::load()` loads the generated browser assets.
- `asset!("./echo.js")` resolves the bundled JavaScript file.
- `new WebSocket(...)` opens the browser connection.
- `WebSocketUpgrade` validates and performs the protocol upgrade.
- `on_upgrade` runs after the WebSocket handshake succeeds.
- `socket.recv()` waits for the next client message.
- `socket.send(message)` sends the same message back.
- Text and binary messages are echoed.
- The browser records both sent and received messages in the page.

Stop the development server by pressing `Ctrl+C`.