Declares a [`Font`] from a family name and its faces.

The family name is a string literal or any expression that converts into a `String`. The macro expands to a [`Font`] that can be stored in a `const`. The faces are built the first time the font is used. With the `discover` feature, the font is also registered for discovery, so [`discover_fonts`] finds it. Without that feature, register the font yourself with [`RouterBuilder::font`].

The faces can be written in one of two forms.

# CSS-like form

Follow the family name with one or more [`@font-face`] blocks, like in a CSS stylesheet. You write the family name once, and the macro adds it to every block. The body of each `@font-face { ... }` block is a [`font_face!`] body without the `font-family` descriptor.

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

Instead of blocks, you can follow the family name with a single expression for the faces. It can be anything that converts into [`FontFaces`], such as a `Vec<FontFace>`. This is useful when you build the faces in code or share them between fonts:

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

In this form the macro does not add the family name to the faces, so each [`FontFace`] must already use the same family.

[`@font-face`]: https://developer.mozilla.org/en-US/docs/Web/CSS/@font-face
[`font_face!`]: macro.font_face.html
[`Font`]: struct.Font.html
[`FontFace`]: struct.FontFace.html
[`FontFaces`]: struct.FontFaces.html
[`discover_fonts`]: trait.RouterBuilderFontExt.html#method.discover_fonts
[`RouterBuilder::font`]: trait.RouterBuilderFontExt.html#method.font
