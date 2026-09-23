Builds a [`FontFace`] from CSS [`@font-face`] syntax. Descriptors are written as `name: value`, separated by semicolons, in any order. `font-family` and `src` are required, and each descriptor may appear only once.

```rust
# use topcoat::font::*;
# fn example() -> FontFace {
font_face! {
    font-family: "Inter";
    src: url("/fonts/inter.woff2") format("woff2");
}
# }
```

Literal values are checked at compile time. Weights outside `100..=900`, malformed or out-of-range angles, code points above `U+10FFFF`, and unknown `format(...)` or `tech(...)` keywords are compile errors.

# Descriptors

## `font-family`

The family name, as a string literal:

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

`src` lists one or more sources, separated by commas, from most to least preferred. Each entry is either `local("Family Name")`, which names a font already installed on the visitor's system, or `url("...")`, which names a font file to download. A `url(...)` entry can carry `format(...)` and `tech(...)` hints, which let the browser skip files it cannot use.

```rust
# use topcoat::font::*;
# fn example() -> FontFace {
font_face! {
    font-family: "Inter";
    src: local("Inter"), url("/fonts/inter.woff2") format("woff2") tech("variations");
}
# }
```

`format(...)` and `tech(...)` are both optional and can be written in either order. Their keywords are checked at compile time against the values CSS defines, so a typo like `format("wof2")` fails to build.

Each argument can also be a Rust expression instead of a literal. For `local(...)`, the expression is converted into a `String`. For `url(...)`, it can be a string or an [`Asset`], whose content-hashed URL is written when the face is rendered. For `format(...)` and `tech(...)`, it must evaluate to a [`FontFormat`] or a [`FontTech`].

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

A single weight, or a range of two weights separated by a space for a variable font. A weight is a number in `100..=900` or one of the keywords `normal` (`400`) and `bold` (`700`):

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

A range covers every weight between its two ends:

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

Keywords and numbers can be mixed in a range, so `font-weight: normal bold` is the same as `400 700`. The second weight must not be lower than the first.

Learn more on [MDN][mdn-font-weight].

## `font-style`

`normal`, `italic`, or `oblique`. An oblique face can have a slant angle, or a range of angles for a variable font. Angles are written in degrees and must be in `-90deg..=90deg`:

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

`oblique` also works without an angle, and with a range of two angles:

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

How text is shown while the face loads: `auto`, `block`, `swap`, `fallback`, or `optional`. Each keyword balances how long text stays invisible against how long a fallback font may be shown before the face replaces it:

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

One or more `U+` ranges, separated by commas, that limit the face to some code points. The browser only downloads the face when the page uses one of them. `U+0041` covers a single code point, and `U+0041-005A` covers every code point from `U+0041` to `U+005A`:

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

Rust cannot read some hexadecimal code points as tokens, such as `000E`. Write these with a `0x` prefix, like `U+0x000E`. Wildcard ranges such as `U+4??` are not supported.

Learn more on [MDN][mdn-unicode-range].

# Rust Expressions

Any descriptor value can be a Rust expression instead of CSS syntax, so a face can be built from data known only at run time. The expression must evaluate to the type for that descriptor:

- `font-family`: a value that converts into a [`String`].
- `src`: a value that converts into [`FontSources`], such as a `Vec<`[`FontSource`]`>`.
- `font-weight`: a [`FontWeightRange`].
- `font-style`: a [`FontStyle`].
- `font-display`: a [`FontDisplay`].
- `unicode-range`: a [`UnicodeRanges`].

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

A value that starts like CSS syntax is read as CSS. For example, a variable named `swap` in `font-display` is read as the `swap` keyword. Wrap such an expression in parentheses to use it as Rust: `font-display: (swap);`.

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
