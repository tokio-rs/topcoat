import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";

import { Breadcrumbs } from "@/components/breadcrumbs";
import { CheckIcon } from "@/components/icons";
import { PALETTE } from "@/components/palette";
import { ProductCard } from "@/components/product-card";
import { ProductImage } from "@/components/product-image";
import { PurchaseForm } from "@/components/purchase-form";
import { filledStars, Stars } from "@/components/stars";
import { formatCount, formatDate, formatMoney, formatRating, isoDate } from "@/lib/format";
import { getStore, maxQuantity, type Product, ratingTenths } from "@/lib/store";
import { listingUrl } from "@/lib/urls";

/** The product with the decimal id in the path, if there is one. */
function findProduct(id: string): Product | undefined {
  return /^[0-9]+$/.test(id) ? getStore().product(Number(id)) : undefined;
}

export async function generateMetadata({
  params,
}: PageProps<"/products/[id]">): Promise<Metadata> {
  const product = findProduct((await params).id);
  return product ? { title: product.name } : {};
}

/** `GET /products/{id}`: product details, specifications, reviews and related products. */
export default async function DetailPage({ params }: PageProps<"/products/[id]">) {
  const product = findProduct((await params).id);
  if (!product) notFound();

  const category = product.categoryRecord;
  const categoryHref = listingUrl({ category: category.slug });
  const rating = formatRating(product.rating);
  const filled = filledStars(ratingTenths(product));
  const compareAt = product.compare_at_cents;

  return (
    <div className="mx-auto max-w-7xl px-4 py-10 sm:px-6 lg:px-8">
      <Breadcrumbs
        trail={[
          { label: "Home", href: "/" },
          { label: "Products", href: "/products" },
          { label: category.name, href: categoryHref },
        ]}
        current={product.name}
      />
      <div className="mt-8 grid gap-10 lg:grid-cols-2 lg:gap-16">
        <div className="lg:sticky lg:top-24 lg:self-start">
          <ProductImage product={product} />
        </div>
        <div>
          <Link
            href={categoryHref}
            className="text-sm font-semibold text-teal-700 hover:text-teal-900"
          >
            {category.name}
          </Link>
          <h1 className="mt-2 text-3xl font-semibold tracking-tight text-balance text-stone-900 sm:text-4xl">
            {product.name}
          </h1>
          <div className="mt-3 flex items-center gap-2">
            <Stars filled={filled} rating={rating} size="large" />
            <span className="text-sm font-medium text-stone-900">{rating}</span>
            <a
              href="#reviews"
              className="text-sm text-stone-500 underline-offset-4 hover:text-stone-900 hover:underline"
            >
              {formatCount(product.review_count)} reviews
            </a>
          </div>
          <div className="mt-6 flex flex-wrap items-baseline gap-3">
            <p className="text-3xl font-semibold tracking-tight text-stone-900">
              {formatMoney(product.price_cents)}
            </p>
            {compareAt !== null && (
              <>
                <s className="text-lg text-stone-400">{formatMoney(compareAt)}</s>
                <span className="rounded-full bg-rose-50 px-2.5 py-1 text-sm font-medium text-rose-700">
                  Save {formatMoney(compareAt - product.price_cents)} (
                  {Math.floor(((compareAt - product.price_cents) * 100) / compareAt)}%)
                </span>
              </>
            )}
          </div>
          <p className="mt-4 text-lg leading-8 text-stone-600">{product.tagline}</p>
          <dl className="mt-6 grid grid-cols-[auto_1fr] gap-x-8 gap-y-3 text-sm">
            <dt className="font-medium text-stone-900">Color</dt>
            <dd className="flex items-center gap-2 text-stone-600">
              <span
                className={`size-4 rounded-full ring-1 ring-black/10 ${PALETTE[product.color].accent}`}
              />
              {PALETTE[product.color].label}
            </dd>
            <dt className="font-medium text-stone-900">Availability</dt>
            <Availability stock={product.stock} />
          </dl>
          {product.stock === 0 ? (
            <div className="mt-8">
              <button
                type="button"
                disabled
                className="w-full cursor-not-allowed rounded-full bg-stone-200 px-8 py-3.5 text-base font-semibold text-stone-500"
              >
                Sold out
              </button>
            </div>
          ) : (
            <PurchaseForm max={maxQuantity(product)} />
          )}
          <ul className="mt-8 space-y-3 border-t border-stone-200 pt-6 text-sm text-stone-600">
            <li className="flex items-center gap-3">
              <CheckIcon />
              Free shipping on orders over $75
            </li>
            <li className="flex items-center gap-3">
              <CheckIcon />
              Free 30-day returns
            </li>
            <li className="flex items-center gap-3">
              <CheckIcon />
              Ships within one business day
            </li>
          </ul>
        </div>
      </div>
      <div className="mt-16 grid gap-12 border-t border-stone-200 pt-16 lg:grid-cols-2">
        <section>
          <h2 className="text-xl font-semibold tracking-tight">Description</h2>
          <div className="mt-4 space-y-4 leading-7 text-stone-600">
            {product.description.split("\n\n").map((paragraph, index) => (
              <p key={index}>{paragraph}</p>
            ))}
          </div>
        </section>
        <section>
          <h2 className="text-xl font-semibold tracking-tight">Specifications</h2>
          <table className="mt-4 w-full text-left text-sm">
            <tbody className="divide-y divide-stone-200 border-y border-stone-200">
              {product.specs.map((spec) => (
                <tr key={spec.label}>
                  <th scope="row" className="w-2/5 py-3 pr-4 font-medium text-stone-900">
                    {spec.label}
                  </th>
                  <td className="py-3 text-stone-600">{spec.value}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      </div>
      <section
        id="reviews"
        className="mt-16 grid gap-10 border-t border-stone-200 pt-16 lg:grid-cols-[18rem_1fr]"
      >
        <div>
          <h2 className="text-xl font-semibold tracking-tight">Customer reviews</h2>
          <div className="mt-4 flex items-center gap-3">
            <p className="text-4xl font-semibold tracking-tight">{rating}</p>
            <Stars filled={filled} rating={rating} size="large" />
          </div>
          <p className="mt-2 text-sm text-stone-600">
            Based on {formatCount(product.review_count)} reviews
          </p>
          <p className="mt-1 text-sm text-stone-500">
            Showing the {product.reviews.length} most recent.
          </p>
        </div>
        <ul className="divide-y divide-stone-200">
          {product.reviews.map((review, index) => (
            <li key={index} className="py-6 first:pt-0">
              <article>
                <div className="flex items-center justify-between gap-4">
                  <Stars filled={review.rating} rating={review.rating} size="small" />
                  <time dateTime={isoDate(review.postedOn)} className="text-sm text-stone-500">
                    {formatDate(review.postedOn)}
                  </time>
                </div>
                <h3 className="mt-3 font-semibold text-stone-900">{review.title}</h3>
                <p className="mt-2 leading-7 text-stone-600">{review.body}</p>
                <p className="mt-3 text-sm font-medium text-stone-900">{review.author}</p>
              </article>
            </li>
          ))}
        </ul>
      </section>
      <section aria-labelledby="related-heading" className="mt-16 border-t border-stone-200 pt-16">
        <h2 id="related-heading" className="text-xl font-semibold tracking-tight">
          You might also like
        </h2>
        <div className="mt-8 grid grid-cols-2 gap-x-6 gap-y-10 lg:grid-cols-4">
          {product.relatedProducts.map((related) => (
            <ProductCard key={related.id} product={related} />
          ))}
        </div>
      </section>
    </div>
  );
}

function Availability({ stock }: { stock: number }) {
  if (stock === 0) return <dd className="font-medium text-stone-500">Sold out</dd>;
  if (stock < 10) return <dd className="font-medium text-amber-700">Only {stock} left in stock</dd>;
  return <dd className="font-medium text-emerald-700">In stock</dd>;
}
