use topcoat::view::{StaticClass, class};

use crate::data::Color;

/// Tailwind classes for a product color.
pub struct Palette {
    /// Background of the product image tile.
    pub tile: StaticClass,
    /// Fill of the product shape and the color swatch.
    pub accent: StaticClass,
}

impl Palette {
    pub fn of(color: Color) -> Self {
        let (tile, accent) = match color {
            Color::Charcoal => (class!("bg-stone-200"), class!("bg-stone-700")),
            Color::Sand => (class!("bg-amber-100"), class!("bg-amber-300")),
            Color::Sage => (class!("bg-green-100"), class!("bg-green-700/70")),
            Color::Navy => (class!("bg-blue-100"), class!("bg-blue-900")),
            Color::Terracotta => (class!("bg-orange-100"), class!("bg-orange-700")),
            Color::Ochre => (class!("bg-yellow-100"), class!("bg-yellow-600")),
            Color::Plum => (class!("bg-purple-100"), class!("bg-purple-800")),
            Color::Sky => (class!("bg-sky-100"), class!("bg-sky-500")),
        };
        Self { tile, accent }
    }
}
