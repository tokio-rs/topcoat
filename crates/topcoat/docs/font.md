Web fonts are loaded through CSS. An [`@font-face`] rule names a font family and tells the browser where to download the font files. With Topcoat, you declare these rules in Rust, the router serves them as a stylesheet, and a component loads that stylesheet into your pages.

# Declaring fonts

[`font!`] declares a [`Font`] from [`@font-face`] blocks you write yourself. The family name comes first, and Topcoat adds it to every block for you. Declare the font as a constant, register it on the router, and load it in the page's `<head>` with the [`link`] component:

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
    // `.discover()` finds the font automatically. You can also register it
    // yourself with `.font(ORBITRON)`.
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

The [`link`] component renders a `<link>` to the stylesheet that holds the font's `@font-face` rules. By default it also preloads the font files, so the browser starts downloading them early.

After that, using the font is plain CSS: any rule on the page can refer to the family by name, like the `style` attribute above. Instead of repeating the name as a string, you can also read it with [`ORBITRON.family()`].

The body of each `@font-face` block uses the syntax of [`font_face!`], which describes every supported descriptor.

## Serving the files as assets

`url(...)` accepts any expression that evaluates to a URL string. It also accepts a Topcoat [`Asset`], so the font file is bundled with your other assets and served from your own origin:

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

A local file works the same way: `url(asset!("./fonts/inter-400.woff2"))`. This needs the `asset` feature and the asset bundle loaded on the router. See the [asset guide] for how bundling works.

# Fontsource

[Fontsource] is an open source catalog of web fonts. It includes every Google Font and many other openly licensed families. [`fontsource_font!`] declares a font straight from the catalog. It checks the family and every requested weight, style, and subset against the catalog at compile time.

Fontsource support needs the `font-fontsource` feature:

```toml
topcoat = { version = "0.8.1", features = ["font-fontsource"] }
```

Then name a family from the [`families`] module. By default, the font includes every weight and style the family ships, but only its default character subset. The browser loads the files from the [jsDelivr] CDN:

```rust
# #[cfg(feature = "font-fontsource")]
# use topcoat::font::{Font, fontsource::fontsource_font};
# #[cfg(feature = "font-fontsource")]
const ROBOTO: Font = fontsource_font!(ROBOTO);
```

The result is a normal [`Font`]. You register it and load it exactly like a font declared with [`font!`].

## Picking weights, styles, and subsets

Every combination of weight, style, and subset is a separate font file, so only include what you use. The `weight`, `style`, and `subset` arguments narrow the font down. Each takes a single value or a list in brackets:

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

See [`fontsource_font!`] for all arguments. To declare a single face instead of a whole font, use [`fontsource_font_face!`].

## Self-hosting Fontsource fonts

By default, the browser loads the font files from the [jsDelivr] CDN. Pass `host: Asset` to bundle the files as Topcoat [assets] instead and serve them from your own origin, with content-hashed URLs:

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
[`font_face!`]: font_face
[`families`]: fontsource/families/index.html
[`fontsource_font!`]: fontsource/macro.fontsource_font.html
[`fontsource_font_face!`]: fontsource/macro.fontsource_font_face.html
[`link`]: link
