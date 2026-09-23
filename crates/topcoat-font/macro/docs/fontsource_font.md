Declares a [`Font`] from the [Fontsource] catalog. The font has one [`FontFace`] for each combination of the weights, styles, and subsets you ask for.

```rust
# use topcoat::font::*;
# use topcoat::font::fontsource::*;
# fn example() -> Font {
fontsource_font!(ROBOTO)
# }
```

With only a family name, the font includes every weight and style the family ships, but only its default subset. Add arguments to choose the weights, styles, and subsets yourself:

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

This font has four faces, one for each combination of weight and subset. Each combination is checked at compile time against the copy of the catalog built into Topcoat.

Like [`font!`], the macro expands to a [`Font`] that can be stored in a `const`, and registers it for discovery when the `discover` feature is enabled.

# Arguments

The **family** comes first, as the name of its [`families`] constant (e.g. `ROBOTO`).

The other arguments are written as `name: value` in any order. `weight`, `style`, and `subset` take a single value or a list in brackets, such as `[400, 700]`:

**`weight`** is a number in `100..=900`. Leave it out to include every weight the family ships.

**`style`** is [`Normal`] or [`Italic`]. Leave it out to include every style the family ships.

**`subset`** is a group of characters, such as [`Latin`] or [`Cyrillic`]. It also sets each face's `unicode-range`, so the browser only downloads a file when the page uses its characters. Leave it out to include only the family's default subset. Unlike weight and style, leaving it out does not include every subset.

**`host`** is where the browser loads the files from, and takes a single value. It defaults to [`JsDelivr`], which loads the files from the [jsDelivr] CDN. Pass [`Asset`] to bundle the files as Topcoat [`Asset`][asset-type]s instead and serve them from your own origin with content-hashed URLs. This needs the `asset` feature.

**`display`** is the [`FontDisplay`] strategy for every face, which controls how text is shown while the font downloads. It takes a single value and defaults to `Swap`.

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

To build a single face, use [`fontsource_font_face!`]. It takes one weight, style, and subset and expands to a [`FontFace`].

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
[`font!`]: ../macro.font.html
