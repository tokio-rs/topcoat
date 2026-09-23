Topcoat UI is a collection of premade components that you can restyle: buttons, cards, form controls, menus, and more. Like [shadcn/ui](https://ui.shadcn.com), you do not use them as a library dependency. Instead, `topcoat ui add` copies the source of a component into your project. From then on it is your code, and you can restyle, rewrite, and extend it.

Each component is a normal `#[component]` function built with `view!`. It is styled with Tailwind utility classes that use a small set of theme tokens, and `topcoat ui init` installs the stylesheet that defines those tokens.

# Setup

You need the [`topcoat` CLI](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/getting_started.md#install-the-cli) and the [Tailwind integration](https://docs.rs/topcoat/latest/topcoat/tailwind/index.html). In addition, enable the `ui` feature:

```toml
[dependencies]
topcoat = { version = "0.8.1", features = ["font-fontsource", "tailwind", "ui"] }

[build-dependencies]
topcoat = { version = "0.8.1", default-features = false, features = ["tailwind"] }
```

## Initialize the package

In the package directory, run:

```sh
topcoat ui init
```

This does two things:

- It creates `components.toml` at the package root. This file records which components you added, from which registry, and at which version. The other `topcoat ui` commands need it. Commit it to version control.
- It installs a theme by writing its CSS to `styles.css` at the package root. If the registry offers only one theme, that theme is installed. If it offers several, you are asked to choose one, or you can name it with `--theme`.

Components are installed into `src/components` by default. Pass `--components-dir` to use another directory. In a workspace, choose the package with `--package <name>`, like `cargo -p`. Every `topcoat ui` subcommand accepts it.

## Wire the theme into Tailwind

The installed `styles.css` is your Tailwind input. It contains the `@import "tailwindcss"` directive, the theme's design tokens, and a `@source` directive that scans `src/**/*.rs` for utility classes. Pass it to the Tailwind build in `build.rs`:

```rust,no_run
fn main() {
    topcoat::tailwind::BuildConfig::new()
        .input("styles.css")
        .render()
        .unwrap();
}
```

## Load the stylesheet and the font

Link the generated stylesheet from your root layout, like in any Tailwind setup. The built-in theme sets `--font-sans` to Geist, but it does not include the font files. The easiest way to load the font is the [Fontsource integration](https://docs.rs/topcoat/latest/topcoat/font/index.html), which is the `font-fontsource` feature from the setup above:

```rust,ignore
use topcoat::{
    Result,
    font::fontsource::fontsource_font,
    router::{Slot, layout},
    tailwind,
    view::{View, view},
};

#[layout]
async fn layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                topcoat::font::link(font: fontsource_font!(GEIST))
                <link rel="stylesheet" href=(tailwind::stylesheet!())>
            </head>
            <body>
                (slot)
            </body>
        </html>
    })
}
```

The theme styles `<body>` with the theme's background, text color, and font, so loading the stylesheet is all you need to do.

# Adding components

Add components by name:

```sh
topcoat ui add button
```

For each component, `topcoat ui add`:

- copies its source into the components directory, for example `src/components/button.rs`,
- adds a `pub mod button;` line to `src/components.rs`, so the module is part of your crate (if you use `src/components/mod.rs` instead, that file is updated; if both files exist, the command fails instead of guessing),
- installs the other components it depends on, and
- records the component in `components.toml`.

You still need to declare the `components` module itself once, with `mod components;` in your crate root.

To see which components are available, run:

```sh
topcoat ui list
```

It lists the components of every registry along with their install status. Pass `--installed` to show only the components you have installed.

# Using components

Installed components are normal modules in your crate. Import them and use them like any other component:

```rust,ignore
mod components;

use components::button::button;
use components::card::{
    card, card_content, card_description, card_footer, card_header, card_title,
};
use components::field::{field, field_description, field_group, field_label};
use components::input::input;
use topcoat::{Result, view::{View, attributes, component, view}};

#[component]
async fn sign_in() -> Result<impl View> {
    Ok(view! {
        card(
            card_header(
                card_title("Sign in")
                card_description("Use your work email to continue.")
            )
            card_content(
                <form id="sign-in" method="post" action="/login">
                    field_group(
                        field(
                            field_label(attrs: attributes! { for="email" }, "Email")
                            input(
                                attrs: attributes! {
                                    id="email"
                                    name="email"
                                    type="email"
                                    placeholder="you@example.com"
                                    aria-describedby="email-description"
                                }
                            )
                            field_description(
                                attrs: attributes! { id="email-description" },
                                "Use the email address associated with your account."
                            )
                        )
                    )
                </form>
            )
            card_footer(
                button(
                    attrs: attributes! { class="w-full" type="submit" form="sign-in" },
                    "Sign in"
                )
            )
        )
    })
}
```

The components follow a few conventions:

- **Variants and sizes are enums.** Components with visual variants take them as props with defaults, for example `button(variant: ButtonVariant::Destructive, size: ButtonSize::Sm, ...)`.
- **Attributes are forwarded.** The `attrs: Attributes` prop, built with the `attributes!` macro, is forwarded to the component's main element. So `id`, `type`, `disabled`, event handlers, and other attributes work as usual. A `class` in `attrs` is added to the component's own classes instead of replacing them.
- **Child content is the content.** Child nodes you pass become the content of the component, so text, icons, and other components can be combined freely.
- **Class strings are reusable.** Components with variants have a `*_variants` function that returns their full class string. Use it to give another element the same look, for example a link that looks like a button:

  ```rust,ignore
  view! {
      <a href="/login" class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))>
          "Sign in"
      </a>
  }
  ```

For everything else, read the component's source: it is in your project now, and its doc comments explain how to use it. The [`ui` example](https://github.com/tokio-rs/topcoat/tree/main/examples/ui) shows every built-in component in a runnable showcase.

# Theming

A theme is a small set of design tokens. These are CSS variables for the page background, text colors, the primary and destructive colors, borders, the focus ring, and shadows. They are defined on `:root`, and again on `.dark` for dark mode. Components only use tokens (`bg-primary`, `text-muted-foreground`, `border-border`, ...), never raw colors, so when you edit the values in `styles.css`, every component changes with them.

`--background` is the page color. `--card` and `--card-foreground` set the background and text color of cards and panels, and `--popover` and `--popover-foreground` do the same for floating menus and popovers. In the neutral theme, these surfaces are slightly lighter than the page, but you can change each color on its own.

Dark mode is opt-in. Put the `dark` class on an element, usually `<html>`, and everything inside it uses the dark values. Hover and press states have no tokens of their own. Components draw them with the fill color at a lower opacity, which works in both light and dark mode.

`init` writes `styles.css` once, and no command changes it after that. Edit it like any other file in your project.

# Updating components

The version of a component is a hash of its source. `components.toml` records the hash when you add a component. `topcoat ui list` compares it with the hash of the registry's current source, and marks a component as having an update when the two differ. Your own edits to the installed file do not count, because only the registry's source is hashed.

To get the new source, add the component again:

```sh
topcoat ui add button --overwrite
```

This replaces your file with the registry's current source. If you changed the component, compare the two versions first and apply your changes again.

# Removing components

Remove a component by name:

```sh
topcoat ui remove button
```

This deletes the component's file, removes its module declaration, and removes it from `components.toml`. Components that were installed as its dependencies stay installed.

# Custom registries

`topcoat ui` can install components from other registries too. A registry is a crate with a `[package.metadata.topcoat-ui]` key. The key points at a directory, relative to the crate root, that contains a `registry.toml` manifest. The manifest lists the themes and components, and their source paths are relative to that directory:

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
# Installed together with data_table. An entry is either the name of a
# component in the same registry, or a table naming a component in another
# registry by that registry's crate name.
dependencies = ["button", { registry = "other-registry-crate", name = "spinner" }]
```

The manifest has no versions or hashes. The version of a component is always the hash of its current source, so to ship an update, publish a new crate version with the changed source.

To use a registry, add its crate as a direct dependency in your `Cargo.toml`. Its components then appear in `topcoat ui list`, and you can add them:

```sh
topcoat ui add data_table --registry my-registry-crate
```

Without `--registry`, `topcoat ui add` uses the built-in registry, named `topcoat`, if it has the component. If only one other registry has it, you are asked to confirm before it is added from there. If several do, pass `--registry` to choose. All registries install into the same components directory, and each file can hold only one component. If a component from another registry already uses the file, you are asked whether to replace it.

The built-in registry is the one exception to the direct dependency rule. The `ui` feature of `topcoat` already depends on its crate, so you do not add it to your `Cargo.toml`.

`topcoat ui init` always installs its theme from the built-in registry.
