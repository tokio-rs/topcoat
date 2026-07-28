# Icon

This example demonstrates how to render inline SVG icons with Topcoat.

It shows how to:

- declare an icon manually from SVG data;
- inherit the surrounding text size and color;
- set a fixed icon size;
- provide an accessible label;
- include icons from an Iconify collection.

## Prerequisites

The first build requires an internet connection to download the Feather icon set from Iconify.

The downloaded set is cached, so subsequent builds can normally run without downloading it again.

## Run the example

From the repository root, run:

```sh
cargo run --manifest-path examples/icon/Cargo.toml
```

From inside the `examples` directory, run:

```sh
cargo run --manifest-path icon/Cargo.toml
```

The application is served by default at:

```text
http://127.0.0.1:3000
```

## Expected result

Open the application in your browser.

The page should display five icons:

1. a trash icon matching the surrounding font size;
2. a crimson trash icon;
3. a larger trash icon with an accessible label;
4. a target icon from Feather;
5. a feather icon from Feather.

All icons are rendered as inline SVG elements.

## Test the page

From another terminal, request the page:

```sh
curl --include http://127.0.0.1:3000/
```

The response should have an HTTP `200` status and contain SVG elements.

Count the rendered SVG elements:

```sh
curl --silent http://127.0.0.1:3000/ \
    | grep --only-matching "<svg" \
    | wc --lines
```

The expected result is:

```text
5
```

## Test in the browser

Inspect the different examples:

- the first trash icon should match the surrounding text size;
- the second trash icon should be crimson;
- the third trash icon should be visibly larger;
- the target and feather icons should render correctly.

Open the browser developer tools and inspect the larger trash icon.

It should have an accessible name based on:

```text
Delete
```

## How it works

- `IconData` stores the SVG view box and body.
- `ViewBox::new(...)` defines the SVG coordinate system.
- `currentColor` makes an icon inherit the text color.
- `icon(data: TRASH)` renders an icon at the default size.
- `size: 48` sets fixed dimensions.
- `label: "Delete"` exposes an accessible name.
- `build.rs` downloads and stages the Feather icon set.
- `iconify::include!("feather")` generates the `feather` Rust module.
- `feather::TARGET` and `feather::FEATHER` refer to staged icons.

Stop the server by pressing `Ctrl+C`.