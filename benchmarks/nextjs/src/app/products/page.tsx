import { Pagination } from "../../components/pagination";
import { ProductCard } from "../../components/product-card";
import { getCategories, getPage, normalizeSort, productsUrl } from "../../lib/catalog";

export const dynamic = "force-dynamic";

const SORT_OPTIONS: [string | null, string][] = [
  [null, "Default"],
  ["name", "Name"],
  ["price", "Price: Low to high"],
  ["price-desc", "Price: High to low"],
  ["rating", "Rating"],
];

const CHIP_ACTIVE = "rounded-full bg-slate-900 px-3 py-1 font-medium text-white";
const CHIP_INACTIVE =
  "rounded-full bg-white px-3 py-1 font-medium text-slate-600 shadow-sm hover:bg-slate-100";

export default async function ProductsPage({
  searchParams,
}: {
  searchParams: Promise<{ page?: string; sort?: string; category?: string }>;
}) {
  const params = await searchParams;
  const sort = normalizeSort(params.sort);
  const category = params.category ?? null;
  const requested = Number.parseInt(params.page ?? "", 10);
  const page = getPage(Number.isNaN(requested) ? 1 : requested, sort, category);

  return (
    <>
      <div className="flex flex-wrap items-baseline justify-between gap-4">
        <h1 className="text-3xl font-bold tracking-tight">All products</h1>
        <p className="text-sm text-slate-500">{`${page.total} products`}</p>
      </div>
      <div className="mt-6 flex flex-wrap items-center gap-2 text-sm">
        <span className="font-medium text-slate-500">Sort:</span>
        {SORT_OPTIONS.map(([value, label]) => (
          <a
            key={label}
            href={productsUrl(1, value, category)}
            className={value === sort ? CHIP_ACTIVE : CHIP_INACTIVE}
          >
            {label}
          </a>
        ))}
      </div>
      <div className="mt-3 flex flex-wrap items-center gap-2 text-sm">
        <span className="font-medium text-slate-500">Category:</span>
        <a href={productsUrl(1, sort, null)} className={category === null ? CHIP_ACTIVE : CHIP_INACTIVE}>
          All
        </a>
        {getCategories().map((entry) => (
          <a
            key={entry.slug}
            href={productsUrl(1, sort, entry.slug)}
            className={category === entry.slug ? CHIP_ACTIVE : CHIP_INACTIVE}
          >
            {entry.name}
          </a>
        ))}
      </div>
      {page.total === 0 ? (
        <p className="mt-8 text-slate-500">No products found.</p>
      ) : (
        <>
          <div className="mt-8 grid grid-cols-2 gap-6 md:grid-cols-3 lg:grid-cols-4">
            {page.items.map((product) => (
              <ProductCard key={product.id} product={product} />
            ))}
          </div>
          <Pagination
            current={page.current}
            pageCount={page.pageCount}
            sort={sort}
            category={category}
          />
        </>
      )}
    </>
  );
}
