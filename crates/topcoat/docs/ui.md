Topcoat UI is a collection of premade, themeable components: buttons, cards, form controls, menus, and more. The components are not consumed as an opaque library dependency. Instead, `topcoat ui add` copies a component's source straight into your project, where it is yours to restyle, rewrite, and extend.

Each component is an ordinary `#[component]` function built on `view!`, styled with Tailwind utility classes that reference a small set of theme tokens. `topcoat ui init` installs the stylesheet that defines those tokens.

# Setup

You need the [`topcoat` CLI](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/getting_started.md#install-the-cli) and the [Tailwind integration](https://docs.rs/topcoat/latest/topcoat/tailwind/index.html). On top of that, enable the `ui` feature:

```toml
[dependencies]
topcoat = { version = "0.4.0", features = ["font-fontsource", "tailwind", "ui"] }

[build-dependencies]
topcoat = { version = "0.4.0", default-features = false, features = ["tailwind"] }
```

## Initialize the package

From the package directory, run:

```sh
topcoat ui init
```

This does two things:

- It creates `components.toml` at the package root: the install state that records which components you have added, from which registry, and at which version. The other `topcoat ui` commands require it. Check it into version control.
- It installs a theme: the theme's CSS is written to `styles.css` at the package root. With a single theme on offer it is installed directly; when a registry offers several you are prompted, or you can name one with `--theme`.

By default components install into `src/components`; pass `--components-dir` to choose another directory. In a workspace, select the package to operate on with `--package <name>` (like `cargo -p`); every `topcoat ui` subcommand accepts it.

## Wire the theme into Tailwind

The installed `styles.css` is your Tailwind input. It carries the `@import "tailwindcss"` directive, the theme's design tokens, and a `@source` directive that scans `src/**/*.rs` for utility classes. Point the Tailwind build at it in `build.rs`:

```rust,no_run
fn main() {
    topcoat::tailwind::BuildConfig::new()
        .input("styles.css")
        .render()
        .unwrap();
}
```

## Load the stylesheet and the font

Link the generated stylesheet from your root layout, as with any Tailwind setup. The built-in theme sets `--font-sans` to Geist, which is not bundled with the theme; the easiest way to provide it is the [Fontsource integration](https://docs.rs/topcoat/latest/topcoat/font/index.html) (the `font-fontsource` feature from the setup above):

```rust,ignore
use topcoat::{
    Result,
    font::fontsource::fontsource_font,
    router::layout,
    tailwind,
    view::view,
};

#[layout]
async fn layout(slot: Result) -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                topcoat::font::link(font: fontsource_font!(GEIST))
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
            </head>
            <body>
                (slot?)
            </body>
        </html>
    }
}
```

The theme's base layer styles `<body>` with the theme's background, text color, and font, so there is nothing to set up beyond loading the stylesheet.

# Adding components

Add components by name:

```sh
topcoat ui add button
```

For each component, `topcoat ui add`:

- copies its source into the components directory (e.g. `src/components/button.rs`),
- declares the module so it is reachable: a `pub mod button;` line is added to `src/components.rs` (or to `src/components/mod.rs` if you keep that module style; when both files exist the command errors rather than guessing),
- installs any components it depends on, and
- records the component in `components.toml`.

To see what is on offer, run:

```sh
topcoat ui list
```

It lists every registry's components along with their install status; `--installed` limits the output to what you have installed.

# Using components

Installed components are ordinary modules in your crate. Import them and use them like any other component:

```rust,ignore
mod components;

use components::button::button;
use components::card::{
    card, card_content, card_description, card_footer, card_header, card_title,
};
use components::input::input;
use components::label::label;
use topcoat::{Result, view::{attributes, component, view}};

#[component]
async fn sign_in() -> Result {
    view! {
        card(
            card_header(
                card_title("Sign in")
                card_description("Use your work email to continue.")
            )
            card_content(
                <form class="flex flex-col gap-2">
                    label(attrs: attributes! { for="email" }, "Email")
                    input(
                        attrs: attributes! { id="email" type="email" placeholder="you@example.com" }
                    )
                </form>
            )
            card_footer(button(attrs: attributes! { class="w-full" }, "Sign in"))
        )
    }
}
```

The components share a few conventions:

- **Variants and sizes are enums.** Components with visual variants take them as props with defaults, e.g. `button(variant: ButtonVariant::Destructive, size: ButtonSize::Sm, ...)`.
- **Attributes are forwarded.** An `attrs: Attributes` prop (built with the `attributes!` macro) is forwarded to the component's underlying element, so `id`, `type`, `disabled`, event handlers, and the rest work as usual. A `class` among them is merged with the component's own classes rather than replacing them.
- **Content is child content.** Anything passed as child nodes becomes the component's content, so text, icons, and other components compose freely.
- **Class strings are reusable.** Components with variants expose a `*_variants` function returning the full class string, for giving another element the same looks, such as a link styled as a button:

  ```rust,ignore
  view! {
      <a href="/login" class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))>
          "Sign in"
      </a>
  }
  ```

Beyond that, each component documents itself: the source now lives in your project, so consult (and adjust) it directly. The [`ui` example](https://github.com/tokio-rs/topcoat/tree/main/examples/ui) shows every built-in component in a runnable showcase.

# Theming

A theme is a small set of design tokens: CSS variables for the page background, text colors, the primary and destructive accents, borders, the focus ring, and control shadows, defined on `:root` and, for dark mode, on `.dark`. Components refer to tokens only (`bg-primary`, `text-muted-foreground`, `border-border`, ...), never to raw colors, so the whole component set restyles itself when you edit the values in `styles.css`.

Dark mode is opt-in: putting the `dark` class on an ancestor (typically `<html>`, or any subtree) switches everything inside it to the dark values. Hover and press states have no tokens of their own; components derive them by applying the fill color at reduced opacity, which adapts to both color schemes automatically.

`styles.css` is installed once by `init` and never touched again; it is yours to edit like any other project file.

# Updating components

A component's version is a hash of its source. `components.toml` records the hash at the time you add a component, and `topcoat ui list` compares it against what the registry currently ships, marking components whose registry source has changed as having an update available. Local edits to your copy do not affect this: only the registry-side source is compared.

To pull the newer source, re-add the component:

```sh
topcoat ui add button --overwrite
```

This replaces your file with the registry's current source, so if you have modified the component, diff first and re-apply your changes.

# Removing components

Remove a component by name:

```sh
topcoat ui remove button
```

This deletes the component's file, removes its module declaration, and drops it from `components.toml`. It does not remove components that were installed as its dependencies.

# Custom registries

`topcoat ui` is not limited to the built-in components. A registry is a crate that carries a `[package.metadata.topcoat-ui]` key pointing at a directory with a `registry.toml` manifest alongside the component sources:

```toml
# Cargo.toml of the registry crate
[package.metadata.topcoat-ui]
registry = "registry"
```

```toml
# registry/registry.toml
version = 1

[themes.acme]
source = "themes/acme.css"

[components.button]
source = "src/components/button.rs"

[components.data_table]
source = "src/components/data_table.rs"
# Installed alongside data_table. An entry names a component in the same
# registry, or one in another registry by that registry's crate name.
dependencies = ["button", { registry = "other-registry-crate", name = "spinner" }]
```

The manifest records no versions or hashes; a component's version is always the hash of its current source, so publishing a new crate version with changed sources is all it takes to ship updates.

To consume a registry, declare its crate as a direct dependency in `Cargo.toml`. Its components then show up in `topcoat ui list` and can be added with:

```sh
topcoat ui add data_table --registry my-registry-crate
```

Without `--registry`, the built-in registry (named `topcoat`) is preferred; when only another registry offers the requested component, you are asked to confirm before pulling from it. All registries install into the same components directory, and a file can hold only one component: adding a component whose file is already occupied by one from another registry offers to replace it.

The built-in registry is the one exception to the direct-dependency rule: the `topcoat` facade pulls its crate in transitively under the `ui` feature, so it needs no entry of its own in your `Cargo.toml`.
