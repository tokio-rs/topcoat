Topcoat UI provides styled components that you can customize. `topcoat ui add` copies their source into your project, where you can edit it directly.

Components use `#[component]`, `view!`, and Tailwind classes. Run `topcoat ui init` to install their theme stylesheet.

# Setup

Install the [`topcoat` CLI](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/getting_started.md#install-the-cli), set up [Tailwind](crate::tailwind), and enable the `ui` feature:

```toml
[dependencies]
topcoat = { version = "0.8.1", features = ["font-fontsource", "tailwind", "ui"] }

[build-dependencies]
topcoat = { version = "0.8.1", default-features = false, features = ["tailwind"] }
```

## Initialize the package

From the package directory, run:

```sh
topcoat ui init
```

This creates `components.toml` to track installed components and `styles.css` for the theme. Commit both files. If several themes are available, the command asks you to choose one. Use `--theme` to select it by name.

Components install into `src/components` by default. Pass `--components-dir` to choose another directory. In a workspace, use `--package <name>` with any `topcoat ui` command to select a package.

## Wire the theme into Tailwind

The installed `styles.css` imports Tailwind, defines the theme, and scans Rust source for utility classes. Use it as the input in `build.rs`:

```rust,no_run
fn main() {
    topcoat::tailwind::BuildConfig::new()
        .input("styles.css")
        .render()
        .unwrap();
}
```

## Load the stylesheet and the font

Link the generated stylesheet from your root layout. Load the font named by `--font-sans` in `styles.css`, or change that token to your own font. This example loads Geist through [Fontsource](crate::font):

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

The stylesheet applies the theme's base colors and font to `<body>`.

# Adding components

Add components by name:

```sh
topcoat ui add button
```

The command copies the component and its dependencies, adds their module declarations, and records them in `components.toml`. With the default directory, `button` installs to `src/components/button.rs` and is declared in `src/components.rs`. An existing `src/components/mod.rs` is also supported, but having both module files is an error.

To see available components, run:

```sh
topcoat ui list
```

The list includes install status. Pass `--installed` to show only installed components.

# Using components

Import installed components from your crate and use them in `view!`. For example, after adding the components used below:

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

The components share a few conventions:

- Use variant and size props to choose a component's appearance.
- Pass HTML attributes through `attrs`, built with `attributes!`. Custom classes are appended to the component's classes.
- Pass child nodes for the component's content.
- Use a component's `*_variants` function, when available, to style another element the same way:

  ```rust,ignore
  view! {
      <a href="/login" class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))>
          "Sign in"
      </a>
  }
  ```

Read each component's source for its props and behavior. The [`ui` example](https://github.com/tokio-rs/topcoat/tree/main/examples/ui) provides a runnable showcase.

# Theming

A theme defines CSS variables in `styles.css`. Components use these variables through Tailwind classes, so editing a token changes the components that use it.

For example, change `--background` to set the page color. Paired tokens such as `--card` and `--card-foreground` control a surface's background and text.

Add the `dark` class to `<html>` or another ancestor to apply the dark theme inside it. Edit the `.dark` values in `styles.css` to customize those colors.

Component updates leave your `styles.css` edits in place.

# Updating components

`topcoat ui list` reports when the registry's source differs from the version you installed. It compares source hashes recorded in `components.toml`, so local edits do not affect update detection.

To pull the newer source, re-add the component:

```sh
topcoat ui add button --overwrite
```

This replaces your file with the registry's current source. Save local changes first, then review the diff and reapply them as needed.

# Removing components

Remove a component by name:

```sh
topcoat ui remove button
```

This deletes the component's file, removes its module declaration, and drops it from `components.toml`. It does not remove components that were installed as its dependencies.

# Custom registries

A registry is a crate containing component sources and a `registry.toml` manifest. Set `[package.metadata.topcoat-ui]` to the manifest's directory:

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

Component versions are computed from their source. Publish a new crate version with changed sources to distribute updates.

To consume a registry, declare its crate as a direct dependency in `Cargo.toml`. Its components then show up in `topcoat ui list` and can be added with:

```sh
topcoat ui add data_table --registry my-registry-crate
```

Without `--registry`, the command prefers the built-in `topcoat` registry. It asks for confirmation before using another registry. Registries share the components directory, so installing over another registry's file also asks for confirmation.

The built-in registry is included by Topcoat's `ui` feature and needs no separate dependency.
