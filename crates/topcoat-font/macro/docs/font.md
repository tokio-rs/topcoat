Declares a [`Font`] from a family name and its faces.

The result can be assigned to a constant. With the `discover` feature, [`discover_fonts`] finds the font automatically. Otherwise, register it with [`RouterBuilder::font`].

The faces can be given in one of two forms.

# CSS-like form

Follow the family name with one or more [`@font-face`] blocks. Each block uses [`font_face!`] syntax and inherits the family name:

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

To build faces in Rust, pass an expression that converts into [`FontFaces`], such as a `Vec<FontFace>`:

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

In this form, each [`FontFace`] must already have the matching family name.

[`@font-face`]: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face
[`font_face!`]: macro.font_face.html
[`Font`]: struct.Font.html
[`FontFace`]: struct.FontFace.html
[`FontFaces`]: struct.FontFaces.html
[`discover_fonts`]: trait.RouterBuilderFontExt.html#method.discover_fonts
[`RouterBuilder::font`]: trait.RouterBuilderFontExt.html#method.font
