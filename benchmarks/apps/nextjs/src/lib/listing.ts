import { type Category, type Product, type Sort, SORTS, type Store } from "./store";

/** Number of products on one listing page. */
export const PAGE_SIZE = 24;

/** The raw query parameters of a product listing. Empty values count as absent. */
export interface ListingParams {
  category?: string | null;
  sort?: string | null;
  page?: string | null;
}

/** One page of a product listing. */
export interface Listing {
  category: Category | undefined;
  sort: Sort;
  page: number;
  pageCount: number;
  total: number;
  products: Product[];
}

/** Why listing parameters could not be resolved, with the matching HTTP status. */
export type ListingError =
  | { status: 404; message: "category not found" | "page not found" }
  | { status: 422; message: "invalid sort" | "invalid page" };

export type ListingResult = { ok: true; listing: Listing } | { ok: false; error: ListingError };

/**
 * Resolves listing parameters against the store.
 *
 * Parameters are validated in order: category, then sort, then page.
 */
export function resolveListing(store: Store, params: ListingParams): ListingResult {
  let category: Category | undefined;
  if (params.category) {
    category = store.category(params.category);
    if (!category) return { ok: false, error: { status: 404, message: "category not found" } };
  }

  let sort: Sort = "newest";
  if (params.sort) {
    if (!isSort(params.sort)) return { ok: false, error: { status: 422, message: "invalid sort" } };
    sort = params.sort;
  }

  const sorted = store.sorted(category, sort);
  const total = sorted.length;
  const pageCount = Math.max(1, Math.ceil(total / PAGE_SIZE));

  let page = 1;
  if (params.page) {
    if (!/^[0-9]+$/.test(params.page) || /^0+$/.test(params.page)) {
      return { ok: false, error: { status: 422, message: "invalid page" } };
    }
    // Numbers too large to represent are past any last page.
    page = Number(params.page);
    if (page > pageCount) return { ok: false, error: { status: 404, message: "page not found" } };
  }

  const start = (page - 1) * PAGE_SIZE;
  return {
    ok: true,
    listing: {
      category,
      sort,
      page,
      pageCount,
      total,
      products: sorted.slice(start, start + PAGE_SIZE),
    },
  };
}

function isSort(value: string): value is Sort {
  return (SORTS as readonly string[]).includes(value);
}
