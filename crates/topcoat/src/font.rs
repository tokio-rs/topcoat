#![doc = include_str!("../docs/font.md")]

pub use topcoat_font::*;
pub use topcoat_font_macro::{font, font_face};

/// Font families from the [Fontsource](https://fontsource.org/) catalog.
///
/// See the [Fontsource section](crate::font#fontsource) of the font guide.
#[cfg(feature = "font-fontsource")]
pub mod fontsource {
    pub use topcoat_font::fontsource::*;
    pub use topcoat_font_macro::{fontsource_font, fontsource_font_face};
}
