Constructs a [`FontFace`] using CSS [`@font-face`] syntax. Descriptors are written `name: value`, separated by semicolons, and may appear in any order. `font-family` and `src` are required.

```rust
# use topcoat::font::*;
# fn example() -> FontFace {
font_face! {
    font-family: "Inter";
    src: url("/fonts/inter.woff2") format("woff2");
}
# }
```

Literal values are validated at compile time: weights outside `100..=900`, malformed angles, code points beyond `U+10FFFF`, and unknown `format()` or `tech()` keywords are all rejected before your program builds.

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

## `src`

`src` lists one or more sources, in preference order, separated by commas. Each entry is either `local("Family Name")`, naming a font already installed on the visitor's system, or `url("…")`, a font file to download with optional `format(…)` and `tech(…)` hints the browser uses to skip files it cannot use.

```rust
# use topcoat::font::*;
# fn example() -> FontFace {
font_face! {
    font-family: "Inter";
    src: local("Inter"), url("/fonts/inter.woff2") format("woff2") tech("variations");
}
# }
```

`format(…)` and `tech(…)` are each optional and may be written in either order. Their keywords are checked at compile time against the CSS-defined values, so a typo like `format("wof2")` fails to build.

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

A range covers every weight a variable font carries:

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

## `font-style`

`normal`, `italic`, or `oblique`. An oblique face may carry a slant angle, or an angle range for variable fonts. Angles are validated to `-90deg..=90deg`:

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

# Rust Expressions

Any descriptor value can be a Rust expression instead of the literal CSS form, letting a face be assembled from runtime data. The expression must resolve to the runtime type for that descriptor: a family name convertible into [`String`] for `font-family`, a value convertible into [`FontSources`] (such as a `Vec<`[`FontSource`]`>`) for `src`, a [`FontWeightRange`] for `font-weight`, a [`FontStyle`] for `font-style`, and a [`UnicodeRanges`] for `unicode-range`.

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

The `format(…)` and `tech(…)` hints likewise accept a parenthesized expression resolving to a [`FontFormat`] or [`FontTech`].

[`@font-face`]: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face
[`FontFace`]: struct.FontFace.html
[`FontFormat`]: enum.FontFormat.html
[`FontSource`]: enum.FontSource.html
[`FontSources`]: struct.FontSources.html
[`FontStyle`]: enum.FontStyle.html
[`FontTech`]: enum.FontTech.html
[`FontWeightRange`]: struct.FontWeightRange.html
[`UnicodeRanges`]: struct.UnicodeRanges.html
[`String`]: https://doc.rust-lang.org/std/string/struct.String.html
