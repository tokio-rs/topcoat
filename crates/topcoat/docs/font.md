Web fonts are typically loaded through CSS. A set of [`@font-face`] rules declare a font family and tells the browser where to download the font files. With Topcoat, you can programatically create these font faces and host them on your router.

# Fontsource

[Fontsource] is an open-source catalog of web fonts. It includes every Google Font, plus other openly licensed families. [`fontsource_font!`] declares a font straight from the catalog, and checks the family name and every requested weight, style, and subset against it at compile time.

## Getting started

Fontsource support lives behind the `font-fontsource` feature:

```toml
topcoat = { version = "0.1", features = ["font-fontsource"] }
```

Declare the font you want to use as a constant by referencing its family name. Then, register it on the router, and load it in the page's `<head>`:

```rust,no_run
use topcoat::{
    Result,
    font::{Font, fontsource::fontsource_font},
    router::{Router, RouterBuilderDiscoverExt, page},
    view::view,
};

// Declare the "Roboto" font.
const ROBOTO: Font = fontsource_font!("Roboto");

#[tokio::main]
async fn main() {
    // `.discover()` will automatically find the font. You can also register it manually using `.font(ROBOTO)`.
    let router = Router::builder().discover().build();
    topcoat::start(router).await.unwrap();
}

#[page("/")]
async fn home() -> Result {
    view! {
        <!DOCTYPE html>
        <html>
            <head>
                topcoat::font::link(font: ROBOTO)
            </head>
            <body>
                <h1 style="font-family: 'Roboto'">"Hello!"</h1>
            </body>
        </html>
    }
}
```

The [`link`] component renders the stylesheet `<link>` that carries the font's `@font-face` rules. By default, this will include every weight and style Roboto ships, only in its default character subset, loaded by the browser from the [jsDelivr] CDN.

Using the font is then ordinary CSS: any rule on the page can refer to the family by name, like the `style` attribute in the example does. Rather than repeating the name as a string, you can also get it from [`ROBOTO.family()`].

## Picking weights, styles, and subsets

Every combination of weight, style, and subset is a separate font file, so only include what you use. The `weight`, `style`, and `subset` arguments narrow the font down; each takes a single value or a bracketed list:

```rust
# use topcoat::font::{Font, fontsource::fontsource_font};
const ROBOTO: Font = fontsource_font!(
    "Roboto",
    weight: [400, 700],
    style: Normal,
    subset: [Latin, Cyrillic],
);
```

See [`fontsource_font!`] for the details of each argument.

## Self-hosting the files

By default the Fontsource font files are loaded from the [jsDelivr] CDN by the user's browser. Pass `host: Asset` to download them at build time instead and serve them from your own origin as content-hashed Topcoat [assets]:

```rust,no_run
use topcoat::{
    asset::{AssetBundle, RouterBuilderAssetExt},
    font::{Font, fontsource::fontsource_font},
    router::{Router, RouterBuilderDiscoverExt},
};

const ROBOTO: Font = fontsource_font!("Roboto", host: Asset);

let router = Router::builder()
    .assets(AssetBundle::load().unwrap())
    .discover()
    .build();
```

This needs the `asset` feature and the asset bundle loaded on the router. See the [asset guide] for how bundling works.

# Custom fonts

For typefaces that aren't in the catalog, [`font!`] declares a font from [`@font-face`] blocks you write yourself. The family name comes first and is injected into every block, and the resulting [`Font`] is registered, served, and loaded exactly like a Fontsource one:

```rust
use topcoat::font::{Font, font};

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
```

## Serving the files as assets

`url(…)` accepts expressions that evaluate to URL strings, but also Topcoat [`Asset`]s. This downloads the file at build time and serves it from your own origin:

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

Local files work similarly with the asset system: `url(asset!("./fonts/inter-400.woff2"))`.

# Serving and loading

Every registered font is served as a small, content-hashed stylesheet containing its `@font-face` rules. The hash is derived from the family name and every face setting, so the URL changes when the font changes and responses can be cached indefinitely.

When you use a [`Font`] as an attribute value in [`view!`] (e.g. `<link rel="stylesheet" href=(ROBOTO)>`), it renders as the URL of the CSS file. This allows linking the CSS from your HTML markup. You can also access the font's family name using `ROBOTO.family()`.

The built-in [`link`] component makes this simple, and also adds preloading: by default it renders a `rel="preload"` link for the first source of each face, so the browser starts fetching the font files before it has parsed the stylesheet. Pass `preload: false` to turn that off:

```rust
# use topcoat::{font::{Font, fontsource::fontsource_font}, view::view};
# const ROBOTO: Font = fontsource_font!("Roboto");
# #[topcoat::view::component]
# async fn example() -> topcoat::Result {
view! {
    topcoat::font::link(font: ROBOTO, preload: false)
}
# }
```

[Fontsource]: https://fontsource.org/
[jsDelivr]: https://www.jsdelivr.com/
[`@font-face`]: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face
[`font-display`]: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face/font-display
[asset guide]: crate::asset
[assets]: crate::asset
[`.discover()`]: crate::router::RouterBuilderDiscoverExt::discover
[`Asset`]: crate::asset::Asset
[`Font`]: Font
[`ROBOTO.family()`]: Font::family
[`FontFace`]: FontFace
[`FontFaces`]: FontFaces
[`font!`]: font
[`font_face!`]: font_face
[`fontsource_font!`]: fontsource::fontsource_font
[`fontsource_font_face!`]: fontsource::fontsource_font_face
[`link`]: link
[`view!`]: crate::view::view
