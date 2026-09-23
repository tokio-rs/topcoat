Constructs a [`Font`] from a family in the [Fontsource] catalog.

```rust
# use topcoat::font::*;
# use topcoat::font::fontsource::*;
# fn example() -> Font {
fontsource_font!(ROBOTO)
# }
```

With nothing but a family name you get every weight and style the [Fontsource] family ships, but only for its default subset. You can override which weights, styles, and subsets you want to include by specifying additional parameters:

```rust
# use topcoat::font::*;
# use topcoat::font::fontsource::*;
# fn example() -> Font {
fontsource_font!(
    ROBOTO,
    weight: [400, 700],
    style: Normal,
    subset: [Latin, Cyrillic],
)
# }
```

The resulting font includes one font face per combination of parameters. Each combination is checked against the vendored catalog at compile time.

# Arguments

The **family** comes first, as the name of its [`families`] constant (e.g. `ROBOTO`).

The `weight`, `style`, and `subset` arguments accept a single value or a bracketed list such as `[400, 700]`. The font includes each requested combination.

**`weight`** is a number in `100..=900`. Omit it for every weight the family ships.

**`style`** is [`Normal`] or [`Italic`]. Omit it for every style the family ships.

**`subset`** selects characters such as [`Latin`] or [`Cyrillic`] and sets each face's `unicode-range`. It defaults to the family's default subset.

**`host`** says where the files are loaded from, and takes a single value rather than a list. It defaults to [`JsDelivr`], which links the fonts on the [jsDelivr] CDN. Pass [`Asset`] instead to download them at build time and serve them from your own origin as content-hashed Topcoat [`Asset`][asset-type]s: this needs the `asset` feature.

**`display`** sets the [`FontDisplay`] strategy applied to every face: how text is shown while the font downloads. It takes a single value rather than a list, and defaults to `Swap`.

```rust
# use topcoat::font::*;
# use topcoat::font::fontsource::*;
# fn example() -> Font {
fontsource_font!(ROBOTO, display: Optional)
# }
```

```rust
# use topcoat::font::*;
# use topcoat::font::fontsource::*;
# fn example() -> Font {
fontsource_font!(ROBOTO, host: Asset)
# }
```

# Single faces

Use [`fontsource_font_face!`] to create a single [`FontFace`].

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
[`fontsource_font_face!`]: macro.fontsource_font_face.html
