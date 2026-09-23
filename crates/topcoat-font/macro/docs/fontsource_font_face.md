Builds a [`FontFace`] for a single font file from the [Fontsource] catalog. The file is picked by its family, weight, style, and subset. The family comes first, as the name of its [`families`] constant. The other arguments are written as `name: value` in any order. `weight` and `style` are required. `subset`, `display`, and `host` are optional.

```rust
# use topcoat::font::*;
# use topcoat::font::fontsource::*;
# fn example() -> FontFace {
fontsource_font_face!(
    ROBOTO,
    weight: 400,
    style: Normal,
)
# }
```

Every value is checked at compile time against the copy of the catalog built into Topcoat.

# Arguments

The **family** comes first, as the name of its [`families`] constant (e.g. `ROBOTO`).

**`weight`** is a single number in `100..=900`. The family must ship that weight.

**`style`** is [`Normal`] or [`Italic`]. The family must ship that style.

**`subset`** is the group of characters the face covers, such as [`Latin`] or [`Cyrillic`]. It also sets the face's `unicode-range`, so the browser only downloads the file when the page uses those characters. Leave it out to use the family's default subset.

**`display`** is the face's [`FontDisplay`] strategy, which controls how text is shown while the font downloads. It defaults to `Swap`.

**`host`** is where the browser loads the file from. It defaults to [`JsDelivr`], which loads the file from the [jsDelivr] CDN. Pass [`Asset`] to bundle the file as a Topcoat [`Asset`][asset-type] instead and serve it from your own origin with a content-hashed URL. This needs the `asset` feature.

```rust
# use topcoat::font::*;
# use topcoat::font::fontsource::*;
# fn example() -> FontFace {
fontsource_font_face!(
    ROBOTO,
    weight: 700,
    style: Italic,
    subset: Cyrillic,
    display: Optional,
    host: Asset,
)
# }
```

# Whole families

This macro builds one face at a time. To declare a font with several weights, styles, or subsets at once, use [`fontsource_font!`]. It builds one face for every combination and expands to a [`Font`].

[Fontsource]: https://fontsource.org/
[jsDelivr]: https://www.jsdelivr.com/
[`families`]: families/index.html
[asset-type]: ../../asset/struct.Asset.html
[`Asset`]: enum.Host.html#variant.Asset
[`JsDelivr`]: enum.Host.html#variant.JsDelivr
[`Normal`]: enum.Style.html#variant.Normal
[`Italic`]: enum.Style.html#variant.Italic
[`Latin`]: enum.Subset.html#variant.Latin
[`Cyrillic`]: enum.Subset.html#variant.Cyrillic
[`FontDisplay`]: ../enum.FontDisplay.html
[`Font`]: ../struct.Font.html
[`FontFace`]: ../struct.FontFace.html
[`fontsource_font!`]: macro.fontsource_font.html
