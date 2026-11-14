namespace Harbor.Infrastructure;

/// <summary>Tailwind classes and the label for a product color.</summary>
/// <param name="Label">The color's display name.</param>
/// <param name="Tile">Background of the product image tile.</param>
/// <param name="Accent">Fill of the product shape and the color swatch.</param>
public sealed record Palette(string Label, string Tile, string Accent)
{
    private static readonly Palette Charcoal = new("Charcoal", "bg-stone-200", "bg-stone-700");
    private static readonly Palette Sand = new("Sand", "bg-amber-100", "bg-amber-300");
    private static readonly Palette Sage = new("Sage", "bg-green-100", "bg-green-700/70");
    private static readonly Palette Navy = new("Navy", "bg-blue-100", "bg-blue-900");
    private static readonly Palette Terracotta = new("Terracotta", "bg-orange-100", "bg-orange-700");
    private static readonly Palette Ochre = new("Ochre", "bg-yellow-100", "bg-yellow-600");
    private static readonly Palette Plum = new("Plum", "bg-purple-100", "bg-purple-800");
    private static readonly Palette Sky = new("Sky", "bg-sky-100", "bg-sky-500");

    public static Palette Of(string color) => color switch
    {
        "charcoal" => Charcoal,
        "sand" => Sand,
        "sage" => Sage,
        "navy" => Navy,
        "terracotta" => Terracotta,
        "ochre" => Ochre,
        "plum" => Plum,
        "sky" => Sky,
        _ => throw new ArgumentOutOfRangeException(nameof(color), color, "unknown product color"),
    };
}
