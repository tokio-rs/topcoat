Declares a [`Font`] from a family name and its faces, and registers it for discovery.

The returned [`Font`] can be stored in a constant. With the `discover` feature, [`discover_fonts`] registers it on the router. Otherwise, register it with [`RouterBuilder::font`].

The faces can be given in one of two forms.

# CSS-like form

Follow the family name with one or more [`@font-face`] blocks. Each uses [`font_face!`] syntax with the family name supplied by the macro:

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

To build faces in Rust, pass an expression convertible into [`FontFaces`]:

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
