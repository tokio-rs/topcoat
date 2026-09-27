Creates one [`FontFace`] from the [Fontsource] catalog. Write the family name first, followed by `name: value` arguments in any order. Include `weight` and `style` to select the face:

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

Every value is checked against the vendored catalog at compile time.

# Arguments

The **family** comes first, as the name of its [`families`] constant (e.g. `ROBOTO`).

**`weight`** is one of the family's available weights in `100..=900`.

**`style`** is [`Normal`] or [`Italic`], and the family has to offer it.

**`subset`** selects characters such as [`Latin`] or [`Cyrillic`] and determines the face's `unicode-range`. It defaults to the family's default subset.

**`display`** sets how text appears while the font loads. It accepts a [`FontDisplay`] value and defaults to `Swap`.

**`host`** selects where the font file is loaded from. It defaults to [`JsDelivr`]. Use [`Asset`] to bundle the file as a Topcoat [asset][asset-type] and serve it yourself. This requires the `asset` feature.

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

Use [`fontsource_font!`] to create a [`Font`] with several weights, styles, or subsets.

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
