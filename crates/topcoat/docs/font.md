Declare web fonts in Rust and serve their CSS through the router. Each [`@font-face`] rule tells the browser which font file to load and when to use it.

# Declaring fonts

[`font!`] declares a font family and its [`@font-face`] rules. Write the family name once, followed by the rules. Register the font on the router and add its stylesheet to the page's `<head>`:

```rust,no_run
use topcoat::{
    Result,
    font::{Font, font},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::{View, view},
};

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
    // Discover the font and page.
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

The [`link`] component loads the font's stylesheet. To register a font manually, call [`font`](RouterBuilderFontExt::font) on the router builder.

Use the family name in CSS to apply the font. [`ORBITRON.family()`] returns that name when you need it in Rust.

## Serving the files as assets

Pass an [`Asset`] to `url(...)` to bundle the font file and serve it from your application:

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

Use `url(asset!("./fonts/inter-400.woff2"))` for a local file. Enable the `asset` feature and load the bundle on the router as described in the [asset guide].

# Fontsource

[Fontsource] is a catalog of open-source web fonts. [`fontsource_font!`] declares a font from this catalog and checks the requested family, weights, styles, and character subsets at compile time.

Fontsource support lives behind the `font-fontsource` feature:

```toml
topcoat = { version = "0.9.0", features = ["font-fontsource"] }
```

Choose a font from [`families`]. By default, the declaration includes all its weights and styles in its default character subset. The browser loads the files from [jsDelivr]:

```rust
# #[cfg(feature = "font-fontsource")]
# use topcoat::font::{Font, fontsource::fontsource_font};
# #[cfg(feature = "font-fontsource")]
const ROBOTO: Font = fontsource_font!(ROBOTO);
```

The resulting [`Font`] can be registered, served, and loaded exactly like a custom one.

## Picking weights, styles, and subsets

Each combination of weight, style, and subset needs a separate font file. Include only the combinations you use. Each argument accepts one value or a bracketed list:

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
