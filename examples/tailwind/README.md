# Tailwind

This example demonstrates how to use Tailwind CSS in a Topcoat application.

It shows how to:

- generate CSS from utility classes used in Rust source files;
- run Tailwind from a Cargo build script;
- serve the generated stylesheet through Topcoat assets;
- use responsive, state, and important utility variants;
- verify that the generated stylesheet loaded successfully.

## Prerequisites

This example uses generated assets and Topcoat's Tailwind integration.

Install the local Topcoat CLI from the repository root if it is not already installed:

```sh
cargo install --path crates/topcoat-cli --locked
```

## Run the example

From the repository root, enter the example directory:

```sh
cd examples/tailwind
```

Start the development server:

```sh
topcoat dev
```

The application is served by default at:

```text
http://127.0.0.1:3000
```

## Expected result

Open the application in your browser.

The page should have:

- a light gray background;
- a centered white card;
- rounded corners and a shadow;
- a green `Tailwind is working` badge;
- a styled blue link.

The following warning must not be visible:

```text
Tailwind is not working: this page should look styled.
```

If the warning is visible or the page has only browser-default styling, the generated stylesheet was not loaded.

## Test the page

Request the root route:

```sh
curl --include http://127.0.0.1:3000/
```

The response should have an HTTP `200` status.

Check that the generated stylesheet is linked:

```sh
curl --silent http://127.0.0.1:3000/ \
    | grep --only-matching --extended-regexp 'href="[^"]+\.css"'
```

The output should contain the URL of a generated CSS asset.

Extract and request the stylesheet:

```sh
stylesheet_url=$(
    curl --silent http://127.0.0.1:3000/ \
        | grep --only-matching --extended-regexp 'href="[^"]+\.css"' \
        | head --lines 1 \
        | cut --delimiter='"' --fields=2
)

echo "$stylesheet_url"

curl --head "http://127.0.0.1:3000$stylesheet_url"
```

The stylesheet request should return an HTTP `200` response and a CSS content type.

## Test the hover state

Move the pointer over **Read the Tailwind docs**.

The button background should change because its classes include:

```text
hover:bg-blue-500
```

## How it works

- `build.rs` runs `BuildConfig::render()`.
- The Tailwind integration scans the project for utility classes.
- Tailwind generates only the CSS required by the project.
- `tailwind::stylesheet!()` returns the generated stylesheet asset.
- `AssetBundle::load()` loads the generated assets.
- `.assets(...)` registers the routes used to serve them.
- `hidden` hides the failure warning when Tailwind works.
- `flex!` overrides the inline `display: none` declaration.
- State variants such as `hover:` generate interactive styles.

Stop the development server by pressing `Ctrl+C`.