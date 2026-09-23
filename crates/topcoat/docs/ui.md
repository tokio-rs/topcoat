Topcoat UI provides components you can copy into your project and edit. Run `topcoat ui add` to install a component, then change its source to suit your app.

Components use `#[component]`, `view!`, and Tailwind classes. Their colors and other shared styles come from a theme stylesheet installed by `topcoat ui init`.

# Setup

Set up the [`topcoat` CLI](https://github.com/tokio-rs/topcoat/blob/main/crates/topcoat/docs/getting_started.md#install-the-cli) and the [Tailwind integration](crate::tailwind), then enable the `ui` feature. This example also uses Fontsource to load the theme's font:

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

The command creates `components.toml` to track installed components and writes the theme to `styles.css`. Check both files into version control. Run `init` before using the other UI commands.

If several themes are available, the command asks you to choose one. Pass `--theme <name>` to select it directly.

Components install into `src/components` by default. Use `--components-dir` to choose another directory. In a workspace, use `--package <name>` with any `topcoat ui` subcommand to select the package.

## Wire the theme into Tailwind

Use the installed `styles.css` as your Tailwind input. It imports Tailwind, defines the theme, and tells Tailwind to scan your Rust source for classes. Configure `build.rs` to use it:

```rust,no_run
fn main() {
    topcoat::tailwind::BuildConfig::new()
        .input("styles.css")
        .render()
        .unwrap();
}
```

## Load the stylesheet and the font

Load the generated stylesheet in your root layout. If the theme names a web font, load that font too. For example, use the [Fontsource integration](crate::font) to load Geist:

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

The theme applies its base styles to `<body>` when the stylesheet loads.

# Adding components

Add components by name:

```sh
topcoat ui add button
```

The command copies the source into your components directory, adds its module declaration, and records the installation in `components.toml`. It also installs any components the requested component depends on.

Module declarations go in `src/components.rs`, or in `src/components/mod.rs` if that file already exists. If both files exist, resolve the conflict before adding components.

To see what is on offer, run:

```sh
topcoat ui list
```

The list shows available components and their install status. Use `--installed` to show only installed components.

# Using components

Installed components are ordinary modules in your crate. Import them and use them like any other component:

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

- **Variants and sizes use enum props.** For example, pass `variant: ButtonVariant::Destructive` to style a destructive action.
- **Extra attributes go in `attrs`.** Build them with `attributes!`. Each component documents which element receives them. Classes are added to the existing classes without resolving conflicting Tailwind utilities. Use a more specific selector or edit the component to override a default reliably.
- **Pass content as children.** Children can contain text, HTML, and other components.
- **Reuse styles through class helpers.** For example, `button_variants` can give a link the appearance of a button:

  ```rust,ignore
  view! {
      <a href="/login" class=(button_variants(ButtonVariant::Outline, ButtonSize::Md))>
          "Sign in"
      </a>
  }
  ```

Read each component's source for its props and behavior. The [`ui` example](https://github.com/tokio-rs/topcoat/tree/main/examples/ui) provides a runnable showcase.

# Theming

A theme defines shared styles with CSS variables in `styles.css`. Components use classes such as `bg-primary` and `text-muted-foreground` to read those variables. Edit the values to change the theme across your app.

For example, `--background` sets the page color, while `--card` and `--card-foreground` set card colors. These values can be adjusted independently.

Add the `dark` class to `<html>` to enable dark mode for the page, or to another ancestor to enable it for a subtree. Edit the `.dark` variables in `styles.css` to change its colors.

The UI commands preserve `styles.css` after initialization, so you can customize it freely.

# Updating components

`topcoat ui list` reports an update when a component's registry source has changed since installation. It compares the current registry hash with the hash saved in `components.toml`. Edits to your local copy do not affect this comparison.

To pull the newer source, re-add the component:

```sh
topcoat ui add button --overwrite
```

This replaces your file with the registry's source. Save any local changes first, then review the diff and reapply the changes you want to keep.

# Removing components

Remove a component by name:

```sh
topcoat ui remove button
```

This deletes the component's file, removes its module declaration, and drops it from `components.toml`. It does not remove components that were installed as its dependencies.

# Custom registries

A custom registry is a Cargo crate that provides component sources and a `registry.toml` manifest. Point to the manifest's directory in the crate's metadata:

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

Component versions are computed from their source. To distribute an update, publish a new version of the registry crate with the changed source.

To consume a registry, declare its crate as a direct dependency in `Cargo.toml`. Its components then show up in `topcoat ui list` and can be added with:

```sh
topcoat ui add data_table --registry my-registry-crate
```

Without `--registry`, the command prefers the built-in `topcoat` registry. It asks for confirmation if the component is available only from another registry. All registries share the same install directory. If a component would replace a file from another registry, the command asks before replacing it.

The built-in registry is available through Topcoat's `ui` feature and needs no separate dependency.
