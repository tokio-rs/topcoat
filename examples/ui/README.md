# UI

This example demonstrates how to build a component library with Topcoat UI.

Topcoat UI components are vendored into the application source code instead of being used as an opaque external component library. The installed components can therefore be modified, restyled, and extended directly inside the project.

The showcase includes:

- badges;
- buttons and button variants;
- cards;
- checkboxes;
- dropdown menus and submenus;
- inputs and labels;
- progress indicators;
- selects;
- spinners;
- switches;
- textareas;
- light and dark theme examples.

## Prerequisites

This example uses:

- Topcoat UI;
- Tailwind CSS;
- Fontsource;
- Iconify;
- generated asset bundles.

Install the local Topcoat CLI from the repository root if it is not already installed:

```sh
cargo install --path crates/topcoat-cli --locked
```

The first build requires an internet connection to download the font and Feather icon set.

The example is already initialized and contains:

```text
components.toml
styles.css
src/components/
```

You do not need to run `topcoat ui init`.

## Run the example

From the repository root, enter the example directory:

```sh
cd examples/ui
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

The page should display a styled component showcase headed by:

```text
Build your component library
```

It should contain multiple cards arranged in responsive columns.

Check that you can see examples including:

- button variants and sizes;
- sign-in and project forms;
- deployment status badges;
- a progress bar at 62%;
- notification controls;
- a dropdown menu with a submenu;
- a destructive delete action;
- a dark deployment card;
- loading spinners;
- `Tabs`, `Dialog`, and `Avatar` placeholders marked as coming soon.

The page should use the Geist font and the neutral Topcoat UI theme.

## Test the page

From another terminal, check the root route:

```sh
curl --include http://127.0.0.1:3000/
```

The response should have an HTTP `200` status.

Check that the page contains the main heading:

```sh
curl --silent http://127.0.0.1:3000/ \
    | grep --fixed-strings "Build your component library"
```

Check that the generated stylesheet is linked:

```sh
curl --silent http://127.0.0.1:3000/ \
    | grep --only-matching --extended-regexp 'href="[^"]+\.css"'
```

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

## Inspect the installed components

From inside `examples/ui`, list the installed UI components:

```sh
topcoat ui list --installed
```

The installed component files are stored in:

```text
src/components/
```

Their installation state and registry hashes are recorded in:

```text
components.toml
```

These files belong to the application and can be modified like any other project source.

## Run the registry synchronization tests

From the repository root, run:

```sh
cargo test --manifest-path examples/ui/Cargo.toml
```

The tests verify that:

- the installed neutral theme matches the built-in registry;
- the vendored components match their registry versions;
- all components currently available in the registry are included in the example.

## How it works

- `components.toml` records the installed theme and components.
- `src/components/` contains the vendored Rust component source.
- `styles.css` defines the neutral theme and its design tokens.
- `build.rs` generates the Tailwind stylesheet and stages the Feather icons.
- `AssetBundle::load()` loads the generated assets.
- `fontsource_font!(GEIST, host: Asset)` self-hosts the Geist font.
- `tailwind::stylesheet!()` returns the generated stylesheet URL.
- `iconify_icon!` includes icons from the staged Feather collection.
- component variant functions return reusable Tailwind class strings.
- the `.dark` class switches a subtree to the dark theme tokens.

Stop the development server by pressing `Ctrl+C`.