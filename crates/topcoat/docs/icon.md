Icons are small vector graphics that sit inside text, like the trash can on a delete button or the magnifier in a search field. Topcoat renders icons as inline `<svg>` elements. They need no extra network requests, scale with the surrounding font, and take on the text color.

# Declaring icons

[`IconData`] holds everything needed to draw an icon: a view box and the SVG markup inside it. Declare an icon as a constant and render it with the [`icon`] component:

```rust,no_run
use topcoat::{
    Result,
    icon::{IconData, icon},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{View, svg::ViewBox, view},
};

// An icon is a view box plus the raw SVG markup inside it.
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

The rendered `<svg>` is `1em` wide and high by default, so the icon matches the font size of the text around it. Wherever its markup uses `currentColor`, it takes on the text color.

Without a `label`, the icon is hidden from assistive technology, which suits icons that only decorate nearby text. With a `label`, the label becomes the icon's accessible name. Pass `size` to set a fixed size instead of following the font, and `attrs` to add extra attributes to the `<svg>` element:

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

[Iconify] is an open catalog of icons. It publishes many open source icon sets in one shared JSON format. Topcoat can use icons from it directly and checks every icon name at compile time.

Iconify support needs the `icon-iconify` feature, both in your dependencies and in your build dependencies:

```toml
[dependencies]
topcoat = { version = "0.8.1", features = ["icon-iconify"] }

[build-dependencies]
topcoat = { version = "0.8.1", default-features = false, features = ["icon-iconify"] }
```

A build script stages the icon sets you use. Add a `build.rs` next to `Cargo.toml` and list the sets by their Iconify prefix. Each set is downloaded on the first build and then cached, so later builds work offline:

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

[`include!`] then turns a staged set into a module of `IconData` constants. Each constant is named after its icon in `SCREAMING_SNAKE_CASE` and renders like any other icon:

```rust,ignore
use topcoat::icon::{icon, iconify};

iconify::include!("feather");

view! {
    icon(data: feather::TRASH_2, label: "Delete")
}
```

## Single icons

[`iconify_icon!`] expands a single `"set:icon"` reference to an [`IconData`] expression. You can use it directly in a view or store it under a name of your choosing:

```rust,ignore
use topcoat::icon::{IconData, iconify};

const TRASH: IconData = iconify::iconify_icon!("feather:trash-2");
```

## Caching and vendoring

By default, downloaded sets are cached in Topcoat's cache inside the Cargo target directory, which every package in the workspace shares. Call [`cache_dir`] to keep the cache in a directory of your own instead. Call [`icon_set_version`] to pin a set to a specific version:

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

Each set is cached at `<dir>/<set>.json`. It is downloaded only when the file is missing or was downloaded for a different pinned version. Commit the directory for offline, reproducible builds, or ignore it in git to keep a cache that survives `cargo clean`. Files you place there yourself are used as they are, so you can add an icon set that is not on Iconify the same way.

[Iconify]: https://iconify.design/
[`IconData`]: IconData
[`icon`]: icon
[`include!`]: iconify/macro.include.html
[`iconify_icon!`]: iconify/macro.iconify_icon.html
[`icon_set_version`]: iconify/struct.BuildConfig.html#method.icon_set_version
[`cache_dir`]: iconify/struct.BuildConfig.html#method.cache_dir
