"""Tailwind classes for each product color.

The class strings are literals so the Tailwind CLI finds them when it scans this file.
"""

from typing import NamedTuple


class Palette(NamedTuple):
    label: str
    # Background of the product image tile.
    tile: str
    # Fill of the product shape and the color swatch.
    accent: str


PALETTES = {
    "charcoal": Palette("Charcoal", "bg-stone-200", "bg-stone-700"),
    "sand": Palette("Sand", "bg-amber-100", "bg-amber-300"),
    "sage": Palette("Sage", "bg-green-100", "bg-green-700/70"),
    "navy": Palette("Navy", "bg-blue-100", "bg-blue-900"),
    "terracotta": Palette("Terracotta", "bg-orange-100", "bg-orange-700"),
    "ochre": Palette("Ochre", "bg-yellow-100", "bg-yellow-600"),
    "plum": Palette("Plum", "bg-purple-100", "bg-purple-800"),
    "sky": Palette("Sky", "bg-sky-100", "bg-sky-500"),
}
