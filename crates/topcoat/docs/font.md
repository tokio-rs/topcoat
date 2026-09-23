Declare web fonts in Rust and load them through CSS [`@font-face`] rules served by your router.

# Declaring fonts

Use [`font!`] to declare a family and its font faces. Register the font on the router, then load it in the page's `<head>`:

```rust,no_run
use topcoat::{
    Result,
    font::{Font, font},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{View, view},
};

// Declare the "Orbitron" font.
const ORBITRON: Font = font! {
    "Orbitron",
    @font-face {
        src: url(
            "https://cdn.jsdelivr.net/fontsource/fonts/orbitron:vf@latest/latin-wght-normal.woff2"
        ) format("woff2") tech("variations");
        font-weight: 100 900;
        font-display: swap;
    }
};

#[tokio::main]
async fn main() {
    // `.discover()` will automatically find the font. You can also register it manually using `.font(ORBITRON)`.
    let router = Router::builder().discover().build();
    topcoat::start(router).await.unwrap();
}

#[page("/")]
async fn home() -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html>
            <head>
                topcoat::font::link(font: ORBITRON)
            </head>
            <body>
                <h1 style="font-family: 'Orbitron'">"Hello!"</h1>
            </body>
        </html>
    })
}
```

The [`link`] component renders the stylesheet `<link>` that carries the font's `@font-face` rules.

Refer to the family by name in CSS, as the example's `style` attribute does. [`ORBITRON.family()`] returns that name when you need it in Rust.

## Serving the files as assets

Pass an [`Asset`] to `url(...)` to download the file during asset bundling and serve it from your application:

```rust
use topcoat::{asset::asset, font::{Font, font}};

const INTER: Font = font! {
    "Inter",
    @font-face {
        src: url(asset!(
            "https://cdn.jsdelivr.net/fontsource/fonts/inter@latest/latin-400-normal.woff2"
        )) format("woff2");
        font-weight: 400;
    }
};
```

Local files work similarly with the asset system: `url(asset!("./fonts/inter-400.woff2"))`. This needs the `asset` feature and the asset bundle loaded on the router. See the [asset guide] for how bundling works.

# Fontsource

[Fontsource] is a web font catalog. [`fontsource_font!`] declares a font from it and checks the requested font faces at compile time.

Fontsource support lives behind the `font-fontsource` feature:

```toml
topcoat = { version = "0.8.1", features = ["font-fontsource"] }
```

Choose a font from [`families`]. By default, the macro includes every available weight and style in the family's default character subset. The browser loads the files from [jsDelivr]:

```rust
# #[cfg(feature = "font-fontsource")]
# use topcoat::font::{Font, fontsource::fontsource_font};
# #[cfg(feature = "font-fontsource")]
const ROBOTO: Font = fontsource_font!(ROBOTO);
```

The resulting [`Font`] can be registered, served, and loaded exactly like a custom one.

## Picking weights, styles, and subsets

Every combination of weight, style, and subset is a separate font file, so only include what you use. The `weight`, `style`, and `subset` arguments narrow the font down; each takes a single value or a bracketed list:

```rust
# #[cfg(feature = "font-fontsource")]
# use topcoat::font::{Font, fontsource::fontsource_font};
# #[cfg(feature = "font-fontsource")]
const ROBOTO: Font = fontsource_font!(
    ROBOTO,
    weight: [400, 700],
    style: Normal,
    subset: [Latin, Cyrillic],
);
```

See [`fontsource_font!`] for the details of each argument.

## Self-hosting Fontsource fonts

Pass `host: Asset` to bundle the font files as Topcoat [assets] and serve them from your application:

```rust,no_run
# #[cfg(feature = "font-fontsource")]
# fn example() {
use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    font::{Font, fontsource::fontsource_font},
    router::{Router, RouterBuilderDiscoverExt},
};

const ROBOTO: Font = fontsource_font!(ROBOTO, host: Asset);

let router = Router::builder()
    .assets(AssetBundle::load().unwrap())
    .discover()
    .build();
# }
```

[Fontsource]: https://fontsource.org/
[jsDelivr]: https://www.jsdelivr.com/
[`@font-face`]: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face
[asset guide]: crate::asset
[assets]: crate::asset
[`Asset`]: crate::asset::Asset
[`Font`]: Font
[`ORBITRON.family()`]: Font::family
[`font!`]: font
[`families`]: fontsource/families/index.html
[`fontsource_font!`]: fontsource/macro.fontsource_font.html
[`link`]: link
