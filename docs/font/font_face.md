Creates a [`FontFace`] with CSS [`@font-face`] syntax. Write descriptors as `name: value;` in any order. Include at least `font-family` and `src`:

```rust
# use topcoat::font::*;
# fn example() -> FontFace {
font_face! {
    font-family: "Inter";
    src: url("/fonts/inter.woff2") format("woff2");
}
# }
```

Literal values are checked at compile time. Invalid values, such as an out-of-range weight or an unknown format, cause a compile error.

# Descriptors

## `font-family`

The family name, given as a string literal:

```rust
# use topcoat::font::*;
# fn example() -> FontFace {
font_face! {
    font-family: "Inter";
    src: local("Inter");
}
# }
```

Learn more on [MDN][mdn-font-family].

## `src`

List sources in preference order, separated by commas. Use `local("Family Name")` for an installed font or `url("...")` for a font file to download. Optional `format(...)` and `tech(...)` hints help the browser skip unsupported files:

```rust
# use topcoat::font::*;
# fn example() -> FontFace {
font_face! {
    font-family: "Inter";
    src: local("Inter"), url("/fonts/inter.woff2") format("woff2") tech("variations");
}
# }
```

The `format(...)` and `tech(...)` hints may appear in either order. Literal keywords are checked at compile time.

These arguments also accept Rust expressions. Pass a family name to `local(...)`, a URL to `url(...)`, a [`FontFormat`] to `format(...)`, or a [`FontTech`] to `tech(...)`. A `url(...)` argument can also be an [`Asset`], resolved when the face is rendered. Sources containing expressions are built at runtime.

```rust
# use topcoat::font::*;
# fn example(installed: String, href: String) -> FontFace {
font_face! {
    font-family: "Inter";
    src: local(installed), url(href) format("woff2");
}
# }
```

Learn more on [MDN][mdn-src].

## `font-weight`

A single weight, or a space-separated range carried by a variable font. Weights are the numbers `100..=900` or the keywords `normal` (`400`) and `bold` (`700`):

```rust
# use topcoat::font::*;
# fn example() -> FontFace {
font_face! {
    font-family: "Inter";
    src: local("Inter");
    font-weight: 400;
}
# }
```

A range declares the weights available from a variable font:

```rust
# use topcoat::font::*;
# fn example() -> FontFace {
font_face! {
    font-family: "Inter";
    src: local("Inter");
    font-weight: 100 900;
}
# }
```

Keywords may be mixed with numbers in a range, so `font-weight: normal bold` is equivalent to `400 700`.

Learn more on [MDN][mdn-font-weight].

## `font-style`

Use `normal`, `italic`, or `oblique`. An oblique face can include a slant angle or, for a variable font, an angle range. Angles must be within `-90deg..=90deg`:

```rust
# use topcoat::font::*;
# fn example() -> FontFace {
font_face! {
    font-family: "Inter";
    src: local("Inter");
    font-style: oblique 14deg;
}
# }
```

A bare `oblique` keyword and an angle range are both accepted:

```rust
# use topcoat::font::*;
# fn example() -> FontFace {
font_face! {
    font-family: "Inter";
    src: local("Inter");
    font-style: oblique 0deg 12deg;
}
# }
```

Learn more on [MDN][mdn-font-style].

## `font-display`

Controls how text appears while the font loads. For example, `swap` lets the browser show fallback text until the font is ready:

```rust
# use topcoat::font::*;
# fn example() -> FontFace {
font_face! {
    font-family: "Inter";
    src: local("Inter");
    font-display: swap;
}
# }
```

Learn more on [MDN][mdn-font-display].

## `unicode-range`

One or more `U+` ranges, separated by commas, restricting the face to a subset of code points. A bare `U+0041` covers a single code point; `U+0041-005A` covers an inclusive range:

```rust
# use topcoat::font::*;
# fn example() -> FontFace {
font_face! {
    font-family: "Inter";
    src: local("Inter");
    unicode-range: U+0000-00FF, U+0131, U+2000-206F;
}
# }
```

Learn more on [MDN][mdn-unicode-range].

# Rust Expressions

Use Rust expressions to build descriptors from runtime data. Each expression must match the descriptor's type:

| Descriptor | Type |
|---|---|
| `font-family` | A value convertible into [`String`] |
| `src` | A value convertible into [`FontSources`], such as `Vec<FontSource>` |
| `font-weight` | [`FontWeightRange`] |
| `font-style` | [`FontStyle`] |
| `font-display` | [`FontDisplay`] |
| `unicode-range` | [`UnicodeRanges`] |

```rust
# use topcoat::font::*;
# fn example() -> FontFace {
let family = String::from("Inter");
let sources = vec![FontSource::local("Inter")];

font_face! {
    font-family: family;
    src: sources;
    font-weight: FontWeightRange::from_u16(400, 700);
}
# }
```

[`@font-face`]: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face
[mdn-font-family]: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face/font-family
[mdn-src]: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face/src
[mdn-font-weight]: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face/font-weight
[mdn-font-style]: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face/font-style
[mdn-font-display]: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face/font-display
[mdn-unicode-range]: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face/unicode-range
[`Asset`]: ../asset/struct.Asset.html
[`FontDisplay`]: enum.FontDisplay.html
[`FontFace`]: struct.FontFace.html
[`FontFormat`]: enum.FontFormat.html
[`FontSource`]: enum.FontSource.html
[`FontSources`]: struct.FontSources.html
[`FontStyle`]: enum.FontStyle.html
[`FontTech`]: enum.FontTech.html
[`FontWeightRange`]: struct.FontWeightRange.html
[`UnicodeRanges`]: struct.UnicodeRanges.html
[`String`]: https://doc.rust-lang.org/std/string/struct.String.html
