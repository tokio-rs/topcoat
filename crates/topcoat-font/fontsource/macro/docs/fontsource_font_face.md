Constructs a [`FontFace`] for one face from the [Fontsource] catalog, picked out by family name and the weight, style, and subset that narrow it to a single font file. The family comes first as a string literal; the rest are `name: value` arguments in any order. `weight` and `style` are required, `subset`, `display`, and `host` are optional.

```rust
# use topcoat::font::*;
# use topcoat::font::fontsource::*;
# fn example() -> FontFace {
fontsource_font_face!(
    "Roboto",
    weight: 400,
    style: Normal,
)
# }
```

Every value is checked against the vendored catalog as your program compiles.

# Arguments

The **family** comes first, as a string literal, and has to match a family in the catalog.

**`weight`** is a single number in `100..=900` — only the weights the family ships are accepted.

**`style`** is [`Normal`] or [`Italic`], and the family has to offer it.

**`subset`** selects the block of characters the face covers, such as [`Latin`] or [`Cyrillic`]; it also determines the face's `unicode-range`. Leave it off to get the family's default subset.

**`display`** sets the face's [`FontDisplay`] strategy — how text is shown while the font downloads. It defaults to `Swap`.

**`host`** says where the file is loaded from. It defaults to [`JsDelivr`], which links the font on the [jsDelivr] CDN. Pass [`Asset`] instead to download the file at build time and serve it from your own origin as a content-hashed Topcoat [`Asset`][asset-type] — this needs the `asset` feature.

```rust
# use topcoat::font::*;
# use topcoat::font::fontsource::*;
# fn example() -> FontFace {
fontsource_font_face!(
    "Roboto",
    weight: 700,
    style: Italic,
    subset: Cyrillic,
    display: Optional,
    host: Asset,
)
# }
```

# Whole families

This macro builds one face at a time. To pull in a family across several weights, styles, or subsets at once, reach for [`fontsource_font!`], which takes every weight, style, and subset you give it and expands to a [`Font`] of the resulting faces.

[Fontsource]: https://fontsource.org/
[jsDelivr]: https://www.jsdelivr.com/
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
