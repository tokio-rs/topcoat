Topcoat renders icons as inline `<svg>` elements. Icons scale with the surrounding text and need no separate network request.

# Declaring icons

[`IconData`] holds an icon's view box and SVG body. Declare it as a constant, then render it with the [`icon`] component:

```rust,no_run
use topcoat::{
    Result,
    icon::{IconData, icon},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{View, svg::ViewBox, view},
};

// An icon is a view box plus a raw SVG body:
const TRASH: IconData = IconData::unescaped_unchecked(
    ViewBox::new(0.0, 0.0, 24.0, 24.0),
    r#"<path fill="currentColor" d="M19,4H15.5L14.5,3H9.5L8.5,4H5V6H19V4M6,19A2,2 0 0,0 8,21H16A2,2 0 0,0 18,19V7H6V19Z"/>"#,
);

#[tokio::main]
async fn main() {
    topcoat::start(Router::builder().discover().build())
        .await
        .unwrap();
}

#[page("/")]
async fn home() -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <body>
                <p>"Move to " icon(data: TRASH, label: "trash")</p>
            </body>
        </html>
    })
}
```

Icons are `1em` square by default. Their SVG bodies can use `currentColor` to inherit the text color. Pass `size` to set the dimensions and `attrs` to add attributes to the `<svg>` element.

An icon without a `label` is hidden from assistive technology. Provide a label when the icon conveys information that the surrounding text does not:

```rust
# use topcoat::{Result, icon::{IconData, icon}, view::*};
# const TRASH: IconData = IconData::unescaped_unchecked(svg::ViewBox::new(0.0, 0.0, 24.0, 24.0), "<g/>");
# #[component]
# async fn example() -> Result<impl View> {
Ok(view! {
    icon(data: TRASH, size: 48, label: "Delete")
})
# }
```

# Iconify

[Iconify] publishes open-source icon sets in a shared format. Topcoat can download these sets and check icon references at compile time.

Iconify support lives behind the `icon-iconify` feature, for both your runtime dependency and your build dependency:

```toml
[dependencies]
topcoat = { version = "0.9.0", features = ["icon-iconify"] }

[build-dependencies]
topcoat = { version = "0.9.0", default-features = false, features = ["icon-iconify"] }
```

Add a `build.rs` next to `Cargo.toml` to stage the sets you use. Each set is downloaded when it is missing from the cache:

```rust,no_run
# #[cfg(feature = "icon-iconify")]
# #[allow(clippy::needless_doctest_main)]
fn main() {
    topcoat::icon::iconify::BuildConfig::new()
        .icon_set("feather")
        .stage()
        .unwrap();
}
# #[cfg(not(feature = "icon-iconify"))]
# fn main() {}
```

[`include!`] creates `IconData` constants from a staged set. The constants use the icons' names in `SCREAMING_SNAKE_CASE`:

```rust,ignore
use topcoat::icon::{icon, iconify};

iconify::include!("feather");

view! {
    icon(data: feather::TRASH_2, label: "Delete")
}
```

## Single icons

Use [`iconify_icon!`] to refer to one icon as an expression. You can pass it directly to a view or assign it to a constant:

```rust,ignore
const TRASH: IconData = iconify::iconify_icon!("feather:trash-2");
```

## Caching and vendoring

By default the downloaded sets are cached in Topcoat's cache inside the Cargo target directory, shared across the workspace. Pass [`cache_dir`] to keep the cache in a directory of your own instead:

```rust,no_run
# #[cfg(feature = "icon-iconify")]
# #[allow(clippy::needless_doctest_main)]
fn main() {
    topcoat::icon::iconify::BuildConfig::new()
        .cache_dir("icons")
        .icon_set("feather")
        .icon_set_version("mdi", "1.30.0")
        .stage()
        .unwrap();
}
# #[cfg(not(feature = "icon-iconify"))]
# fn main() {}
```

Each set is cached at `<dir>/<set>.json` and downloaded only when its file is missing or pinned to a different version. Commit the directory for offline, reproducible builds, or gitignore it to keep a cache that survives `cargo clean`. Files you place there yourself are used as-is, so an icon set that is not on Iconify can be vendored the same way.

[Iconify]: https://iconify.design/
[`IconData`]: IconData
[`icon`]: icon
[`include!`]: iconify/macro.include.html
[`iconify_icon!`]: iconify/macro.iconify_icon.html
[`icon_set_version`]: iconify/struct.BuildConfig.html#method.icon_set_version
[`cache_dir`]: iconify/struct.BuildConfig.html#method.cache_dir
