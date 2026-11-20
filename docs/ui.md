Topcoat UI provides components you can copy into your project and edit. Run `topcoat ui add` to install a component, then change its source to suit your app.

Components use `#[component]`, `view!`, and Tailwind classes. Their colors and other shared styles come from a theme stylesheet installed by `topcoat ui init`.

# Setup

Set up the [`topcoat` CLI](https://github.com/tokio-rs/topcoat/blob/main/docs/getting_started.md#install-the-cli) and the [Tailwind integration](crate::tailwind), then enable the `ui` feature. This example also uses Fontsource to load the theme's font:

```toml
[dependencies]
topcoat = { version = "0.12.1", features = ["font-fontsource", "tailwind", "ui"] }

[build-dependencies]
topcoat = { version = "0.12.1", default-features = false, features = ["tailwind"] }
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
    context::Cx,
    font::fontsource::fontsource_font,
    router::{Slot, layout},
    tailwind,
    view::{View, view},
};

#[layout]
async fn layout(cx: &Cx, slot: Slot<'_>) -> Result<impl View> {
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

To install every component the registry offers, pass `--all`:

```sh
topcoat ui add --all
```

Components that are already installed are skipped and reported. Add `--overwrite` to replace them with the registry's source as well.

To see what is on offer, run:

```sh
topcoat ui list
```

The list shows available components and their install status, and the installed theme. Use `--installed` to show only what is installed.

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
use topcoat::{
    Result,
    context::Cx,
    view::{View, attributes, component, element_id, view},
};

#[component]
async fn sign_in(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        // Elements that refer to each other share these IDs.
        let form = element_id(cx);
        let email = element_id(cx);
        let email_description = element_id(cx);

        card(
            card_header(
                card_title("Sign in")
                card_description("Use your work email to continue.")
            )
            card_content(
                <form id=(&form) method="post" action="/login">
                    field_group(
                        field(
                            field_label(attrs: attributes! { for=(&email) }, "Email")
                            input(
                                attrs: attributes! {
                                    id=(&email)
                                    name="email"
                                    type="email"
                                    placeholder="you@example.com"
                                    aria-describedby=(&email_description)
                                }
                            )
                            field_description(
                                attrs: attributes! { id=(&email_description) },
                                "Use the email address associated with your account."
                            )
                        )
                    )
                </form>
            )
            card_footer(
                button(
                    attrs: attributes! { class="w-full" type="submit" form=(&form) },
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

The theme has no shadows by default. To enable them, replace `--shadow-xs` and `--shadow-sm` in both `:root` and `.dark` with the commented values beside them in `styles.css`.

You can customize `styles.css` freely. When the registry's theme changes, `topcoat ui update` merges the changes into your file instead of replacing it. See [Updating components](#updating-components).

# Updating components

To pull updates, first update the registry crate. For the built-in registry, that is `topcoat` itself. Then run `topcoat ui list` to see which components have an update available. To apply the updates, run:

```sh
topcoat ui update
```

The command updates the theme and every installed component. A file you have not edited is replaced with the new source. A file you have edited is merged, so your changes are kept.

Name components to update only those, or pass `--theme` to update the theme. Add `--dry-run` to see what would happen without writing any file:

```sh
topcoat ui update button card
topcoat ui update --theme
```

If you and the registry changed the same lines, the command leaves both versions in the file between conflict markers and exits with a non-zero status:

```text
<<<<<<< ours
class="h-9 rounded-full px-4"
=======
class="h-9 rounded-md px-6"
>>>>>>> theirs
```

`ours` is your version and `theirs` is the registry's. Edit the file to keep what you want, then delete the markers.

So that a merge can always be undone, the command refuses to merge when the git repository has uncommitted changes, or when the package is not in one. Commit your work first, or pass `--allow-dirty` or `--allow-no-vcs`.

To drop your changes to a component and take the registry's source instead, run `topcoat ui add button --overwrite`.

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
