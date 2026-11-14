import type { Color } from "@/lib/store";

/** Tailwind classes and the label for a product color. */
export interface Palette {
  label: string;
  /** Background of the product image tile. */
  tile: string;
  /** Fill of the product shape and the color swatch. */
  accent: string;
}

export const PALETTE: Record<Color, Palette> = {
  charcoal: { label: "Charcoal", tile: "bg-stone-200", accent: "bg-stone-700" },
  sand: { label: "Sand", tile: "bg-amber-100", accent: "bg-amber-300" },
  sage: { label: "Sage", tile: "bg-green-100", accent: "bg-green-700/70" },
  navy: { label: "Navy", tile: "bg-blue-100", accent: "bg-blue-900" },
  terracotta: { label: "Terracotta", tile: "bg-orange-100", accent: "bg-orange-700" },
  ochre: { label: "Ochre", tile: "bg-yellow-100", accent: "bg-yellow-600" },
  plum: { label: "Plum", tile: "bg-purple-100", accent: "bg-purple-800" },
  sky: { label: "Sky", tile: "bg-sky-100", accent: "bg-sky-500" },
};
