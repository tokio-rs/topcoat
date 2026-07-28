# Font

This example demonstrates two ways to use web fonts with Topcoat.

It shows how to:

- select a font from the Fontsource catalog;
- download and self-host a font through Topcoat assets;
- declare an `@font-face` rule manually;
- load a font directly from an external CDN;
- generate font preload and style elements.

## Prerequisites

This example uses generated assets.

Install the local Topcoat CLI from the repository root if it is not already installed:

```sh
cargo install --path crates/topcoat-cli --locked
```

The first build requires an internet connection to download the Fontsource files.

The browser also needs an internet connection to load the Orbitron font from jsDelivr.

## Run the example

From the repository root, enter the example directory:

```sh
cd examples/font
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

The page displays two lines using visibly different fonts.

The first line uses Lavishly Yours:

```text
This font is downloaded from Fontsource and self-hosted via Topcoat assets!
```

The second line uses Orbitron:

```text
This font is declared by hand and loaded straight from the jsDelivr CDN!
```

## Inspect the font requests

Open the browser developer tools and select the **Network** panel.

Reload the page and filter the requests by:

```text
woff2
```

The Lavishly Yours font should be served by the local Topcoat application from a URL under:

```text
/_topcoat/assets/
```

The Orbitron font should be requested from:

```text
cdn.jsdelivr.net
```

Both font requests should complete successfully.

## Test the page route

From another terminal, run:

```sh
curl --include http://127.0.0.1:3000/
```

The response should have an HTTP `200` status and contain both text lines.

You can check them directly with:

```sh
curl --silent http://127.0.0.1:3000/ \
    | grep --extended-regexp "Fontsource|jsDelivr"
```

## How it works

- `fontsource_font!` selects font faces from the Fontsource catalog.
- `host: Asset` downloads the selected files and adds them to the Topcoat asset bundle.
- `font!` declares a font and its `@font-face` rules manually.
- `AssetBundle::load()` loads the generated local assets.
- `topcoat::font::link(...)` generates the elements needed to preload and register a font.
- `.family()` returns the font-family name used by the inline styles.
- Lavishly Yours is served locally by Topcoat.
- Orbitron is loaded directly from jsDelivr.

Stop the development server by pressing `Ctrl+C`.