import type { Metadata } from "next";
import Link from "next/link";
import { notFound } from "next/navigation";

import { Breadcrumbs } from "@/components/breadcrumbs";
import { Pagination } from "@/components/pagination";
import { ProductCard } from "@/components/product-card";
import { formatCount } from "@/lib/format";
import { type ListingParams, PAGE_SIZE, resolveListing } from "@/lib/listing";
import { type Sort, SORTS, getStore } from "@/lib/store";
import { listingUrl } from "@/lib/urls";

const SIDEBAR_LINK =
  "flex items-center justify-between gap-3 whitespace-nowrap rounded-lg px-3 py-2 text-sm font-medium text-stone-600 hover:bg-stone-100 hover:text-stone-900";
const SIDEBAR_LINK_CURRENT =
  "flex items-center justify-between gap-3 whitespace-nowrap rounded-lg bg-teal-50 px-3 py-2 text-sm font-semibold text-teal-900";
const SORT_LINK =
  "block rounded-full bg-white px-3 py-1.5 text-sm font-medium text-stone-600 ring-1 ring-inset ring-stone-200 hover:bg-stone-100";
const SORT_LINK_CURRENT =
  "block rounded-full bg-stone-900 px-3 py-1.5 text-sm font-medium text-white ring-1 ring-inset ring-stone-900";

const SORT_LABELS: Record<Sort, string> = {
  newest: "Newest",
  "price-asc": "Price: low to high",
  "price-desc": "Price: high to low",
  rating: "Top rated",
};

const ALL_PRODUCTS = {
  name: "All products",
  description: "Everything in the Harbor Market catalog, from tents to turntables.",
};

type SearchParams = Awaited<PageProps<"/products">["searchParams"]>;

/** The listing parameters from the query string. Repeated parameters use their first value. */
function listingParams(searchParams: SearchParams): ListingParams {
  const first = (value: string | string[] | undefined) =>
    Array.isArray(value) ? value[0] : value;
  return {
    category: first(searchParams.category),
    sort: first(searchParams.sort),
    page: first(searchParams.page),
  };
}

export async function generateMetadata({
  searchParams,
}: PageProps<"/products">): Promise<Metadata> {
  const { category } = listingParams(await searchParams);
  const title = (category && getStore().category(category)?.name) || ALL_PRODUCTS.name;
  return { title };
}

/** `GET /products`: one page of products with category sidebar, sort links and pagination. */
export default async function ListingPage({ searchParams }: PageProps<"/products">) {
  const store = getStore();
  const result = resolveListing(store, listingParams(await searchParams));
  // Invalid `sort` and `page` values are answered with a 422 page by `src/proxy.ts`.
  if (!result.ok) notFound();
  const { listing } = result;

  const { name: title, description } = listing.category ?? ALL_PRODUCTS;
  const currentSlug = listing.category?.slug;
  const sidebarEntries = [
    { slug: undefined, name: ALL_PRODUCTS.name, count: store.products.length },
    ...store.categories.map((category) => ({
      slug: category.slug,
      name: category.name,
      count: category.productCount,
    })),
  ];
  const firstPosition = (listing.page - 1) * PAGE_SIZE + 1;
  const lastPosition = firstPosition + listing.products.length - 1;

  return (
    <div className="mx-auto max-w-7xl px-4 py-10 sm:px-6 lg:px-8">
      {listing.category ? (
        <Breadcrumbs
          trail={[
            { label: "Home", href: "/" },
            { label: "Products", href: "/products" },
          ]}
          current={title}
        />
      ) : (
        <Breadcrumbs trail={[{ label: "Home", href: "/" }]} current="Products" />
      )}
      <div className="mt-6 flex flex-col gap-4 border-b border-stone-200 pb-8 md:flex-row md:items-end md:justify-between">
        <div>
          <h1 className="text-3xl font-semibold tracking-tight sm:text-4xl">{title}</h1>
          <p className="mt-2 max-w-2xl text-stone-600">{description}</p>
        </div>
        <p className="shrink-0 text-sm text-stone-500">
          {listing.total === 0
            ? "0 products"
            : `Showing ${firstPosition} to ${lastPosition} of ${formatCount(listing.total)} products`}
        </p>
      </div>
      <div className="mt-8 flex flex-col gap-10 lg:flex-row">
        <aside className="lg:w-60 lg:shrink-0">
          <h2 className="px-3 text-sm font-semibold text-stone-900">Categories</h2>
          <ul className="-mx-1 mt-3 flex gap-1 overflow-x-auto px-1 pb-2 lg:mx-0 lg:block lg:space-y-1 lg:overflow-visible lg:px-0 lg:pb-0">
            {sidebarEntries.map(({ slug, name, count }) => {
              const href = listingUrl({ category: slug, sort: listing.sort });
              return (
                <li key={name}>
                  {slug === currentSlug ? (
                    <Link href={href} aria-current="page" className={SIDEBAR_LINK_CURRENT}>
                      {name}
                      <span className="text-xs text-teal-700">{formatCount(count)}</span>
                    </Link>
                  ) : (
                    <Link href={href} className={SIDEBAR_LINK}>
                      {name}
                      <span className="text-xs text-stone-400">{formatCount(count)}</span>
                    </Link>
                  )}
                </li>
              );
            })}
          </ul>
        </aside>
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-3">
            <h2 className="text-sm font-medium text-stone-500">Sort by</h2>
            <ul className="flex flex-wrap gap-2">
              {SORTS.map((sort) => {
                const href = listingUrl({ category: currentSlug, sort });
                return (
                  <li key={sort}>
                    {sort === listing.sort ? (
                      <Link href={href} aria-current="true" className={SORT_LINK_CURRENT}>
                        {SORT_LABELS[sort]}
                      </Link>
                    ) : (
                      <Link href={href} className={SORT_LINK}>
                        {SORT_LABELS[sort]}
                      </Link>
                    )}
                  </li>
                );
              })}
            </ul>
          </div>
          {listing.total === 0 ? (
            <p className="mt-8 rounded-2xl border border-dashed border-stone-300 p-12 text-center text-stone-500">
              No products in this category yet.
            </p>
          ) : (
            <div className="mt-8 grid grid-cols-2 gap-x-6 gap-y-10 md:grid-cols-3 xl:grid-cols-4">
              {listing.products.map((product) => (
                <ProductCard key={product.id} product={product} />
              ))}
            </div>
          )}
          {listing.pageCount > 1 && <Pagination listing={listing} />}
        </div>
      </div>
    </div>
  );
}
