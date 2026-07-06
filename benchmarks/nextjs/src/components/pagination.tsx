import { productsUrl } from "../lib/catalog";

const PAGE_LINK = "rounded-md px-3 py-2 font-medium text-slate-600 hover:bg-slate-100";
const PAGE_DISABLED = "rounded-md px-3 py-2 font-medium text-slate-300";
const PAGE_CURRENT = "rounded-md bg-indigo-600 px-3 py-2 font-semibold text-white";

export function Pagination({
  current,
  pageCount,
  sort,
  category,
}: {
  current: number;
  pageCount: number;
  sort: string | null;
  category: string | null;
}) {
  const numbers = Array.from({ length: pageCount }, (_, index) => index + 1);

  return (
    <nav
      aria-label="Pagination"
      className="mt-10 flex flex-wrap items-center justify-center gap-1 text-sm"
    >
      {current > 1 ? (
        <a href={productsUrl(current - 1, sort, category)} className={PAGE_LINK}>
          Previous
        </a>
      ) : (
        <span className={PAGE_DISABLED}>Previous</span>
      )}
      {numbers.map((number) =>
        number === current ? (
          <span key={number} aria-current="page" className={PAGE_CURRENT}>
            {number}
          </span>
        ) : (
          <a key={number} href={productsUrl(number, sort, category)} className={PAGE_LINK}>
            {number}
          </a>
        ),
      )}
      {current < pageCount ? (
        <a href={productsUrl(current + 1, sort, category)} className={PAGE_LINK}>
          Next
        </a>
      ) : (
        <span className={PAGE_DISABLED}>Next</span>
      )}
    </nav>
  );
}
