import { type Product, formatPrice, formatRating, initials } from "../lib/catalog";
import { RatingStars } from "./rating-stars";

export function ProductCard({ product }: { product: Product }) {
  return (
    <a
      href={`/products/${product.id}`}
      className="group flex flex-col rounded-xl border border-slate-200 bg-white p-4 shadow-sm transition hover:shadow-md"
    >
      <div className="mb-4 flex h-32 items-center justify-center rounded-lg bg-slate-100 text-3xl font-bold text-slate-300">
        {initials(product.name)}
      </div>
      <p className="text-xs font-medium uppercase tracking-wide text-slate-400">
        {product.category}
      </p>
      <h3 className="mt-1 text-sm font-semibold text-slate-900 group-hover:text-indigo-600">
        {product.name}
      </h3>
      <div className="mt-2 flex items-center gap-1">
        <RatingStars tenths={product.rating_tenths} size="h-4 w-4" />
        <span className="text-xs text-slate-500">
          {`${formatRating(product.rating_tenths)} (${product.review_count})`}
        </span>
      </div>
      <p className="mt-3 text-lg font-bold">{formatPrice(product.price_cents)}</p>
    </a>
  );
}
