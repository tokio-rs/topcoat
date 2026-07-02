Constructs a [`Font`] from the [Fontsource] catalog, which can be a whole family's worth of [`FontFace`]s, one for each combination of the weights, styles, and subsets you ask for.

```rust
# use topcoat::font::*;
# use topcoat::font::fontsource::*;
# fn example() -> Font {
fontsource_font!("Roboto")
# }
```

With nothing but a family name you get every weight and style the [Fontsource] family ships, but only for its default subset. You can override which weights, styles, and subsets you want to include by specifying additional parameters:

```rust
# use topcoat::font::*;
# use topcoat::font::fontsource::*;
# fn example() -> Font {
fontsource_font!(
    "Roboto",
    weight: [400, 700],
    style: Normal,
    subset: [Latin, Cyrillic],
)
# }
```

The resulting font includes one font face per combination of parameters. The macro verifies the existence of each font face in the font family at compile time.

# Arguments

The **family** comes first, as a string literal, and has to match a family in the catalog.

The remaining arguments each take either a single value or a bracketed list of them, `[a, b]`, to fan out across:

**`weight`** is a number in `100..=900`. Omit it for every weight the family ships.

**`style`** is [`Normal`] or [`Italic`]. Omit it for every style the family ships.

**`subset`** is a block of characters such as [`Latin`] or [`Cyrillic`], and sets each face's `unicode-range`. Omit it for the family's default subset alone — unlike weight and style, leaving it out does *not* pull in everything.

**`host`** says where the files are loaded from, and takes a single value rather than a list. It defaults to [`JsDelivr`], which links the fonts on the [jsDelivr] CDN. Pass [`Asset`] instead to download them at build time and serve them from your own origin as content-hashed Topcoat [`Asset`][asset-type]s — this needs the `asset` feature.

**`display`** sets the [`FontDisplay`] strategy applied to every face — how text is shown while the font downloads. It takes a single value rather than a list, and defaults to `Swap`.

```rust
# use topcoat::font::*;
# use topcoat::font::fontsource::*;
# fn example() -> Font {
fontsource_font!("Roboto", display: Optional)
# }
```

```rust
# use topcoat::font::*;
# use topcoat::font::fontsource::*;
# fn example() -> Font {
fontsource_font!("Roboto", host: Asset)
# }
```

# Single faces

To manage individual font faces reach for [`fontsource_font_face!`], which takes a single weight, style, and subset and expands to a lone [`FontFace`].

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
[`fontsource_font_face!`]: macro.fontsource_font_face.html
