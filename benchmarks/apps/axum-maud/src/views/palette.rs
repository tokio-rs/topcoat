use crate::data::Color;

/// Tailwind classes for a product color.
pub struct Palette {
    /// Background of the product image tile.
    pub tile: &'static str,
    /// Fill of the product shape and the color swatch.
    pub accent: &'static str,
}

impl Palette {
    pub fn of(color: Color) -> Self {
        let (tile, accent) = match color {
            Color::Charcoal => ("bg-stone-200", "bg-stone-700"),
            Color::Sand => ("bg-amber-100", "bg-amber-300"),
            Color::Sage => ("bg-green-100", "bg-green-700/70"),
            Color::Navy => ("bg-blue-100", "bg-blue-900"),
            Color::Terracotta => ("bg-orange-100", "bg-orange-700"),
            Color::Ochre => ("bg-yellow-100", "bg-yellow-600"),
            Color::Plum => ("bg-purple-100", "bg-purple-800"),
            Color::Sky => ("bg-sky-100", "bg-sky-500"),
        };
        Self { tile, accent }
    }
}
