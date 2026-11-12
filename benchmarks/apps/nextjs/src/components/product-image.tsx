import type { Product } from "@/lib/store";
import { PALETTE } from "./palette";

/**
 * The square product tile with the sale and sold-out badges.
 *
 * The dataset has no photos, so the tile draws a soft shape in the product's color instead.
 */
export function ProductImage({ product }: { product: Product }) {
  const palette = PALETTE[product.color];
  const onSale = product.compare_at_cents !== null;
  const soldOut = product.stock === 0;
  return (
    <div className={`relative aspect-square overflow-hidden rounded-2xl ${palette.tile}`}>
      <div
        className={`absolute left-1/2 top-1/2 aspect-square w-2/5 -translate-x-1/2 -translate-y-1/2 rotate-6 rounded-[28%] shadow-xl transition group-hover:rotate-12 ${palette.accent}`}
      />
      {(onSale || soldOut) && (
        <div className="absolute left-3 top-3 flex gap-1.5">
          {soldOut && (
            <span className="rounded-full bg-stone-900 px-2.5 py-1 text-xs font-medium text-white">
              Sold out
            </span>
          )}
          {onSale && (
            <span className="rounded-full bg-rose-600 px-2.5 py-1 text-xs font-medium text-white">
              Sale
            </span>
          )}
        </div>
      )}
    </div>
  );
}
