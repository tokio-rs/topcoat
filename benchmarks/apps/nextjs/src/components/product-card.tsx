import Link from "next/link";

import { formatCount, formatMoney, formatRating } from "@/lib/format";
import { type Product, ratingTenths } from "@/lib/store";
import { productUrl } from "@/lib/urls";
import { PALETTE } from "./palette";
import { ProductImage } from "./product-image";
import { filledStars, Stars } from "./stars";

/** A product in a grid: tile, category, name, tagline, rating, price and color. */
export function ProductCard({ product }: { product: Product }) {
  const category = product.categoryRecord;
  const palette = PALETTE[product.color];
  const rating = formatRating(product.rating);
  return (
    <article className="group relative flex flex-col">
      <ProductImage product={product} />
      <div className="mt-4 flex flex-1 flex-col">
        <p className="text-xs font-medium uppercase tracking-wide text-stone-500">{category.name}</p>
        <h3 className="mt-1 text-sm font-semibold text-stone-900 group-hover:text-teal-800">
          <Link href={productUrl(product.id)}>
            <span className="absolute inset-0" />
            {product.name}
          </Link>
        </h3>
        <p className="mt-1 line-clamp-2 text-sm text-stone-600">{product.tagline}</p>
        <div className="mt-2 flex items-center gap-1.5">
          <Stars filled={filledStars(ratingTenths(product))} rating={rating} size="small" />
          <span className="text-xs text-stone-500">
            {rating} ({formatCount(product.review_count)})
          </span>
        </div>
        <div className="mt-auto flex items-end justify-between gap-2 pt-3">
          <p className="flex items-baseline gap-2">
            <span className="font-semibold text-stone-900">{formatMoney(product.price_cents)}</span>
            {product.compare_at_cents !== null && (
              <s className="text-sm text-stone-400">{formatMoney(product.compare_at_cents)}</s>
            )}
          </p>
          <span className="flex items-center gap-1.5 text-xs text-stone-500">
            <span className={`size-3 rounded-full ring-1 ring-black/10 ${palette.accent}`} />
            {palette.label}
          </span>
        </div>
        {product.stock >= 1 && product.stock <= 9 && (
          <p className="mt-1 text-xs font-medium text-amber-700">Only {product.stock} left</p>
        )}
      </div>
    </article>
  );
}
