# Runtime

This example demonstrates how to create interactive pages with Topcoat's browser runtime.

It shows how to:

- store reactive state with signals;
- update state from browser event handlers;
- render expressions when signal values change;
- synchronize HTML attributes with state;
- organize runtime pages with the module router.

## Prerequisites

This example uses Topcoat's browser runtime and generated assets.

Install the local Topcoat CLI from the repository root if it is not already installed:

```sh
cargo install --path crates/topcoat-cli --locked
```

## Run the example

From the repository root, enter the example directory:

```sh
cd examples/runtime
```

Start the Topcoat development server:

```sh
cargo topcoat dev
```

The application is served by default at:

```text
http://127.0.0.1:3000
```

Opening the root route redirects to:

```text
http://127.0.0.1:3000/counter
```

## Counter

Open:

```text
http://127.0.0.1:3000/counter
```

The page displays two buttons and a counter starting at `0`.

Click **increment** to increase the value:

```text
0
1
2
```

Click **decrement** to decrease it.

The value updates without reloading the page.

## Show and hide

Open:

```text
http://127.0.0.1:3000/show
```

The message is initially hidden and the button displays:

```text
click to reveal
```

Click the button. The page should display:

```text
hello world!
```

The button label should change to:

```text
click to hide
```

Click it again to hide the message.

## Test the redirect

With the application running, send a request to the root route:

```sh
curl --include http://127.0.0.1:3000/
```

The response should redirect to:

```text
/counter
```

## How it works

- `module_router!()` builds routes from the Rust module structure.
- `AssetBundle::load()` loads the generated browser assets.
- `topcoat::runtime::script()` loads the browser runtime.
- `signal count` stores the counter state in the browser.
- `increment()` and `decrement()` update the counter signal.
- `signal show` stores whether the message is visible.
- `toggle()` switches the Boolean signal.
- `$(...)` renders an expression that reacts to signal changes.
- `@click` attaches a browser event handler.
- `:hidden` keeps the HTML `hidden` attribute synchronized with state.

Stop the development server by pressing `Ctrl+C`.