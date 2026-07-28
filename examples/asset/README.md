# Asset

This example demonstrates how to bundle and serve a local asset with Topcoat.

It shows how to:

- declare a local file with `asset!`;
- generate an asset bundle;
- load the bundle when building the router;
- render the generated asset URL in an HTML page;
- serve the bundled file through the application.

## Prerequisites

This example requires the Topcoat CLI to generate the asset bundle.

Install the local CLI from the repository root if it is not already installed:

```sh
cargo install --path crates/topcoat-cli --locked
```

## Run the example

From the repository root, enter the example directory:

```sh
cd examples/asset
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

The page should display the Ferris image stored at:

```text
src/ferris.png
```

Inspecting the generated HTML should show an image URL similar to:

```text
/_topcoat/assets/ferris-<hash>.png
```

The exact hash depends on the contents of the file.

## Test the asset

Request the page from another terminal:

```sh
curl --silent http://127.0.0.1:3000/
```

The response should contain an image element whose `src` begins with:

```text
/_topcoat/assets/
```

Extract the generated asset URL:

```sh
asset_url=$(
    curl --silent http://127.0.0.1:3000/ \
        | grep --only-matching --extended-regexp '/_topcoat/assets/[^"]+\.png' \
        | head --lines 1
)

echo "$asset_url"
```

Request the bundled image:

```sh
curl --head "http://127.0.0.1:3000$asset_url"
```

The asset request should return an HTTP `200` response and an image content type.

## How it works

- `asset!("./ferris.png")` declares an asset relative to `src/main.rs`.
- The declaration is embedded in the compiled application.
- `topcoat dev` scans the application and generates the asset bundle.
- The generated filename contains a hash based on the file contents.
- `AssetBundle::load()` loads the generated bundle.
- `.assets(...)` registers the routes that serve the bundled files.
- Rendering the asset handle produces its public URL.

The content hash allows assets to use long-lived caching while producing a new URL whenever the file changes.

Stop the development server by pressing `Ctrl+C`.