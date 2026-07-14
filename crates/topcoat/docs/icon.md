Icons on the web are small vector graphics that flow with the text around them: the trash can on a delete button, the magnifier in a search field. Topcoat renders icons as inline `<svg>` elements, so they need no extra network requests, scale with the surrounding font, and follow the text color.

# Declaring icons

[`IconData`] is the renderable data of an icon: a view box plus the SVG body markup. Declare an icon as a constant and render it with the [`icon`] component:

```rust,no_run
use topcoat::{
    Result,
    icon::{IconData, icon},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{svg::ViewBox, view},
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
async fn home() -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <body>
                <p>"Move to " icon(data: TRASH, label: "trash")</p>
            </body>
        </html>
    }
}
```

The rendered `<svg>` is `1em` square by default, so the icon matches the font size of the surrounding text, and it inherits the text color wherever its body uses `currentColor`. Without a `label` the icon is hidden from assistive technology; with one, the label becomes its accessible name. Pass `size` to fix the dimensions instead of following the font, and `attrs` to forward extra attributes to the `<svg>` element:

```rust
# use topcoat::{Result, icon::{IconData, icon}, view::*};
# const TRASH: IconData = IconData::unescaped_unchecked(svg::ViewBox::new(0.0, 0.0, 24.0, 24.0), "<g/>");
# #[component]
# async fn example() -> Result {
view! {
    icon(data: TRASH, size: 48, label: "Delete")
}
# }
```

# Iconify

[Iconify] is an open catalog of icons: over 150 open source icon sets published in one uniform JSON format. Topcoat can pull icons straight from it, checking every icon reference at compile time.

Iconify support lives behind the `icon-iconify` feature, for both your runtime dependency and your build dependency:

```toml
[dependencies]
topcoat = { version = "0.0.3", features = ["icon-iconify"] }

[build-dependencies]
topcoat = { version = "0.0.3", default-features = false, features = ["icon-iconify"] }
```

Icon sets are staged by a build script. Add a `build.rs` next to `Cargo.toml` naming the sets you use; each set downloads on the first build and is cached, so subsequent builds stay offline:

```rust,no_run
# #[allow(clippy::needless_doctest_main)]
fn main() {
    topcoat::icon::iconify::BuildConfig::new()
        .icon_set("feather")
        .stage()
        .unwrap();
}
```

[`include!`] then expands a staged set to `IconData` consts, named after the icons in `SCREAMING_SNAKE_CASE`, and ready to render like any other icon:

```rust,ignore
use topcoat::icon::{icon, iconify};

iconify::include!("feather");

view! {
    icon(data: feather::TRASH_2, label: "Delete")
}
```

## Single icons

[`iconify_icon!`] is the expression form: it expands one `"set:icon"` reference to a const-evaluable [`IconData`] expression, inline in a view or behind a name of your choosing:

```rust,ignore
const TRASH: IconData = iconify::iconify_icon!("feather:trash-2");
```

## Caching and vendoring

By default the downloaded sets are cached in Topcoat's cache inside the Cargo target directory, shared across the workspace. Pass [`cache_dir`] to keep the cache in a directory of your own instead:

```rust,no_run
# #[allow(clippy::needless_doctest_main)]
fn main() {
    topcoat::icon::iconify::BuildConfig::new()
        .cache_dir("icons")
        .icon_set("feather")
        .icon_set_version("mdi", "1.30.0")
        .stage()
        .unwrap();
}
```

Each set is cached at `<dir>/<set>.json` and downloaded only when its file is missing or pinned to a different version. Commit the directory for offline, reproducible builds, or gitignore it to keep a cache that survives `cargo clean`. Files you place there yourself are used as-is, so an icon set that is not on Iconify can be vendored the same way.

[Iconify]: https://iconify.design/
[`IconData`]: IconData
[`icon`]: icon
[`include!`]: iconify/macro.include.html
[`iconify_icon!`]: iconify/macro.iconify_icon.html
[`icon_set_version`]: iconify/struct.BuildConfig.html#method.icon_set_version
[`cache_dir`]: iconify/struct.BuildConfig.html#method.cache_dir
