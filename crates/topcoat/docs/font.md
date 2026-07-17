Web fonts are typically loaded through CSS. A set of [`@font-face`] rules declare a font family and tells the browser where to download the font files. With Topcoat, you can programatically create these font faces and host them on your router.

# Declaring fonts

[`font!`] declares a font from [`@font-face`] blocks you write yourself. The family name comes first and is injected into every block. Declare the font you want to use as a constant, register it on the router, and load it in the page's `<head>`:

```rust,no_run
use topcoat::{
    Result,
    font::{Font, font},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::view,
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
async fn home() -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                topcoat::font::link(font: ORBITRON)
            </head>
            <body>
                <h1 style="font-family: 'Orbitron'">"Hello!"</h1>
            </body>
        </html>
    }
}
```

The [`link`] component renders the stylesheet `<link>` that carries the font's `@font-face` rules.

Using the font is then ordinary CSS: any rule on the page can refer to the family by name, like the `style` attribute in the example does. Rather than repeating the name as a string, you can also get it from [`ORBITRON.family()`].

## Serving the files as assets

`url(...)` accepts expressions that evaluate to URL strings, but also Topcoat [`Asset`]s. This downloads the file at build time and serves it from your own origin:

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

[Fontsource] is an open-source catalog of web fonts. It includes every Google Font, plus other openly licensed families. [`fontsource_font!`] declares a font straight from the catalog, and checks the family and every requested weight, style, and subset against it at compile time.

Fontsource support lives behind the `font-fontsource` feature:

```toml
topcoat = { version = "0.1.2", features = ["font-fontsource"] }
```

Then specify which font from the [`families`] module you would like to use. By default, this will include every weight and style the font ships, only in its default character subset, loaded by the browser from the [jsDelivr] CDN:

```rust
# use topcoat::font::{Font, fontsource::fontsource_font};
const ROBOTO: Font = fontsource_font!(ROBOTO);
```

The resulting [`Font`] can be registered, served, and loaded exactly like a custom one.

## Picking weights, styles, and subsets

Every combination of weight, style, and subset is a separate font file, so only include what you use. The `weight`, `style`, and `subset` arguments narrow the font down; each takes a single value or a bracketed list:

```rust
# use topcoat::font::{Font, fontsource::fontsource_font};
const ROBOTO: Font = fontsource_font!(
    ROBOTO,
    weight: [400, 700],
    style: Normal,
    subset: [Latin, Cyrillic],
);
```

See [`fontsource_font!`] for the details of each argument.

## Self-hosting Fontsource fonts

By default the font files are loaded from the [jsDelivr] CDN by the user's browser. Pass `host: Asset` to download them at build time instead and serve them from your own origin as content-hashed Topcoat [assets]:

```rust,no_run
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
