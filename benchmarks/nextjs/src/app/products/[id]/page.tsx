import { notFound } from "next/navigation";

import { Breadcrumbs } from "../../../components/breadcrumbs";
import { ProductCard } from "../../../components/product-card";
import { RatingStars } from "../../../components/rating-stars";
import { ReviewList } from "../../../components/review-list";
import { SpecTable } from "../../../components/spec-table";
import {
  type Product,
  formatPrice,
  formatRating,
  getProduct,
  initials,
} from "../../../lib/catalog";

export const dynamic = "force-dynamic";

export default async function ProductDetailPage({
  params,
}: {
  params: Promise<{ id: string }>;
}) {
  const { id } = await params;
  const product = getProduct(Number.parseInt(id, 10));
  if (!product) {
    notFound();
  }

  const related = product.related_ids
    .map((relatedId) => getProduct(relatedId))
    .filter((entry): entry is Product => entry !== undefined);

  return (
    <>
      <Breadcrumbs
        category={product.category}
        categorySlug={product.category_slug}
        name={product.name}
      />
      <div className="mt-6 grid gap-10 lg:grid-cols-2">
        <div className="flex min-h-80 items-center justify-center rounded-2xl bg-slate-100 text-7xl font-bold text-slate-300">
          {initials(product.name)}
        </div>
        <div>
          <p className="text-sm font-medium uppercase tracking-wide text-slate-400">
            {product.category}
          </p>
          <h1 className="mt-1 text-3xl font-bold tracking-tight">{product.name}</h1>
          <div className="mt-3 flex items-center gap-2">
            <RatingStars tenths={product.rating_tenths} size="h-5 w-5" />
            <span className="text-sm text-slate-500">
              {`${formatRating(product.rating_tenths)} (${product.review_count} reviews)`}
            </span>
          </div>
          <p className="mt-6 text-4xl font-bold">{formatPrice(product.price_cents)}</p>
          <div className="mt-6 space-y-4 text-slate-600">
            {product.description.map((paragraph, index) => (
              <p key={index}>{paragraph}</p>
            ))}
          </div>
          <button
            type="button"
            className="mt-8 inline-block rounded-lg bg-indigo-600 px-8 py-3 text-sm font-semibold text-white"
          >
            Add to cart
          </button>
        </div>
      </div>
      <section className="mt-16">
        <h2 className="text-2xl font-bold tracking-tight">Specifications</h2>
        <SpecTable specs={product.specs} />
      </section>
      <section className="mt-16">
        <h2 className="text-2xl font-bold tracking-tight">{`Reviews (${product.review_count})`}</h2>
        <ReviewList reviews={product.reviews} />
      </section>
      <section className="mt-16">
        <h2 className="text-2xl font-bold tracking-tight">Related products</h2>
        <div className="mt-6 grid grid-cols-2 gap-6 md:grid-cols-4">
          {related.map((entry) => (
            <ProductCard key={entry.id} product={entry} />
          ))}
        </div>
      </section>
    </>
  );
}
