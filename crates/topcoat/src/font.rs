#![doc = include_str!("../docs/font.md")]

pub use topcoat_font::*;
pub use topcoat_font_macro::*;

#[cfg(feature = "font-fontsource")]
pub mod fontsource {
    pub use topcoat_font_fontsource::*;
    pub use topcoat_font_fontsource_macro::*;
}
