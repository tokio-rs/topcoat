Creates a [`Font`] from the [Fontsource] catalog. The font contains a face for each requested combination of weight, style, and character subset.

```rust
# use topcoat::font::*;
# use topcoat::font::fontsource::*;
# fn example() -> Font {
fontsource_font!(ROBOTO)
# }
```

With only a family name, the font includes every available weight and style in the default subset. Use named arguments to select the combinations you need:

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

Each combination is checked against the bundled catalog at compile time.

# Arguments

The **family** comes first, as the name of its [`families`] constant (e.g. `ROBOTO`).

The `weight`, `style`, and `subset` arguments accept one value or a bracketed list:

**`weight`** is a number in `100..=900`. Omit it for every weight the family ships.

**`style`** is [`Normal`] or [`Italic`]. Omit it for every style the family ships.

**`subset`** selects characters such as [`Latin`] or [`Cyrillic`] and sets each face's `unicode-range`. Omitting it includes only the family's default subset.

**`host`** selects where font files are loaded from. It takes one value and defaults to [`JsDelivr`]. Use [`Asset`] to bundle the files as Topcoat [assets][asset-type] and serve them yourself. This requires the `asset` feature.

**`display`** sets how text appears while the font loads. It takes one [`FontDisplay`] value, applies to every face, and defaults to `Swap`.

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
