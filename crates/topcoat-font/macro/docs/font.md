Declares a [`Font`] from a family name and its faces, and registers it for discovery.

Expands to a `const` [`Font`]. With the `discover` feature, it is also registered so [`discover_fonts`] finds it; without it, register the returned font manually with [`RouterBuilder::font`].

The faces can be given in one of two forms.

# CSS-like form

Follow the family name with one or more CSS [`@font-face`]-like blocks. The family name is given once and injected into every block, so the faces read like a CSS stylesheet without repeating it. Each `@font-face { ... }` block is a [`font_face!`] body (minus its `font-family`).

```rust
# use topcoat::font::{Font, font};
#
const INTER: Font = font! {
    "Inter",
    @font-face {
        src: url("/inter-400.woff2") format("woff2");
        font-weight: 400;
    }
    @font-face {
        src: url("/inter-700.woff2") format("woff2");
        font-weight: 700;
    }
};
```

# Expression form

Alternatively, follow the family name with a single expression that evaluates to the faces: anything convertible into [`FontFaces`], such as a `Vec<FontFace>`. This uses ordinary Rust syntax instead of the CSS-like blocks, which is handy when the faces are built up programmatically or shared between fonts:

```rust
# use topcoat::font::{Font, FontFace, FontFormat, FontSource, font};
#
fn inter_faces() -> Vec<FontFace> {
    vec![FontFace::new(
        "Inter",
        vec![FontSource::url("/inter-400.woff2", Some(FontFormat::Woff2), None)],
    )]
}
const INTER: Font = font!("Inter", inter_faces());
```

Unlike the CSS-like form, the family name is not injected into the faces, so each [`FontFace`] must already carry the matching family.

[`@font-face`]: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face
[`font_face!`]: macro.font_face.html
[`Font`]: struct.Font.html
[`FontFace`]: struct.FontFace.html
[`FontFaces`]: struct.FontFaces.html
[`discover_fonts`]: trait.RouterBuilderFontExt.html#method.discover_fonts
[`RouterBuilder::font`]: trait.RouterBuilderFontExt.html#method.font
