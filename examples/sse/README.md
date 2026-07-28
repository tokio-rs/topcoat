# Server-sent events

This example demonstrates how to stream events from a Topcoat server to the browser with Server-Sent Events.

It includes:

- a continuous stream that sends one tick every second;
- event IDs and reconnection support;
- keep-alive messages;
- a finite stream that reports job progress;
- named SSE events;
- JSON event data.

## Prerequisites

This example uses a bundled JavaScript asset.

Install the local Topcoat CLI from the repository root if it is not already installed:

```sh
cargo install --path crates/topcoat-cli --locked
```

## Run the example

From the repository root, enter the example directory:

```sh
cd examples/sse
```

Start the development server:

```sh
topcoat dev
```

The application is served by default at:

```text
http://127.0.0.1:3000
```

## Test the tick stream

Open the application in your browser.

The page should initially add:

```text
connected
```

A new tick should then appear every second:

```text
tick: 0
tick: 1
tick: 2
```

The stream remains open and continues sending events.

## Test the job stream

Click **Run a job**.

The page should append progress updates:

```text
job: 0%
job: 10%
job: 20%
```

The updates continue until the final message:

```text
job: finished
```

The job stream then closes.

## Test with curl

Connect to the continuous tick stream:

```sh
curl --no-buffer http://127.0.0.1:3000/ticks
```

The response should contain events similar to:

```text
event: tick
id: 0
retry: 1000
data: 0
```

Stop the command with `Ctrl+C`.

Connect to the finite job stream:

```sh
curl --no-buffer http://127.0.0.1:3000/job
```

The stream should produce progress events followed by:

```text
event: done
data: finished
```

## Test reconnection

Request the tick stream with a previous event ID:

```sh
curl --no-buffer \
    --header "Last-Event-ID: 5" \
    http://127.0.0.1:3000/ticks
```

The first event should use ID `6`.

## How it works

- `Sse` creates a Server-Sent Events response.
- `Event` defines an individual event.
- `.event(...)` assigns a named event type.
- `.id(...)` assigns an event ID.
- `.data(...)` provides text data.
- `.json_data(...)` serializes structured data as JSON.
- `.retry(...)` suggests how long the browser should wait before reconnecting.
- `last_event_id(cx)` reads the `Last-Event-ID` request header.
- `KeepAlive` keeps a long-lived connection active.
- `EventSource` opens the SSE connections in the browser.
- The `/ticks` stream runs continuously.
- The `/job` stream ends after sending its `done` event.

Stop the development server by pressing `Ctrl+C`.