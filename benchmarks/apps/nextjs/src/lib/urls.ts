import type { Sort } from "./store";

/** The URL of a product detail page. */
export function productUrl(id: number): string {
  return `/products/${id}`;
}

/** The URL of a listing page. Parameters with default values are left out. */
export function listingUrl({
  category,
  sort = "newest",
  page = 1,
}: {
  category?: string;
  sort?: Sort;
  page?: number;
}): string {
  const params = [];
  if (category) params.push(`category=${category}`);
  if (sort !== "newest") params.push(`sort=${sort}`);
  if (page !== 1) params.push(`page=${page}`);
  return params.length === 0 ? "/products" : `/products?${params.join("&")}`;
}
