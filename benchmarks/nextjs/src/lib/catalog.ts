import data from "../../../data/products.json";

export interface Category {
  name: string;
  slug: string;
}

export interface Spec {
  key: string;
  value: string;
}

export interface Review {
  author: string;
  date: string;
  rating_tenths: number;
  title: string;
  body: string;
}

export interface Product {
  id: number;
  name: string;
  slug: string;
  category: string;
  category_slug: string;
  price_cents: number;
  rating_tenths: number;
  review_count: number;
  featured: boolean;
  specs: Spec[];
  description: string[];
  reviews: Review[];
  related_ids: number[];
}

export const PAGE_SIZE = 24;

const categories = data.categories as Category[];
const products = data.products as Product[];
const byId = new Map(products.map((product) => [product.id, product]));

export function getCategories(): Category[] {
  return categories;
}

export function getProduct(id: number): Product | undefined {
  return byId.get(id);
}

export function getFeatured(): Product[] {
  return products.filter((product) => product.featured);
}

export interface ProductsPage {
  items: Product[];
  current: number;
  pageCount: number;
  total: number;
}

// The parity contract shared by every benchmark app: `category` filters by
// exact slug match, `sort` must already be normalized, all sorts tie-break by
// ascending id, and `page` is clamped to the valid range.
export function getPage(page: number, sort: string | null, category: string | null): ProductsPage {
  const items = products.filter(
    (product) => category === null || product.category_slug === category,
  );

  switch (sort) {
    case "name":
      items.sort((a, b) => (a.name < b.name ? -1 : a.name > b.name ? 1 : a.id - b.id));
      break;
    case "price":
      items.sort((a, b) => a.price_cents - b.price_cents || a.id - b.id);
      break;
    case "price-desc":
      items.sort((a, b) => b.price_cents - a.price_cents || a.id - b.id);
      break;
    case "rating":
      items.sort((a, b) => b.rating_tenths - a.rating_tenths || a.id - b.id);
      break;
  }

  const total = items.length;
  const pageCount = Math.max(1, Math.ceil(total / PAGE_SIZE));
  const current = Math.min(Math.max(page, 1), pageCount);
  const start = (current - 1) * PAGE_SIZE;

  return { items: items.slice(start, start + PAGE_SIZE), current, pageCount, total };
}

export function normalizeSort(sort: string | undefined): string | null {
  switch (sort) {
    case "name":
    case "price":
    case "price-desc":
    case "rating":
      return sort;
    default:
      return null;
  }
}

// Canonical query parameter order (page, sort, category), omitting defaults.
export function productsUrl(page: number, sort: string | null, category: string | null): string {
  let url = "/products";
  let sep = "?";

  if (page > 1) {
    url += `${sep}page=${page}`;
    sep = "&";
  }
  if (sort !== null) {
    url += `${sep}sort=${sort}`;
    sep = "&";
  }
  if (category !== null) {
    url += `${sep}category=${category}`;
  }

  return url;
}

export function formatPrice(cents: number): string {
  return `$${Math.floor(cents / 100)}.${String(cents % 100).padStart(2, "0")}`;
}

export function formatRating(tenths: number): string {
  return `${Math.floor(tenths / 10)}.${tenths % 10}`;
}

export function filledStars(tenths: number): number {
  return Math.floor((tenths + 5) / 10);
}

export function initials(name: string): string {
  return name
    .split(/\s+/)
    .slice(0, 2)
    .map((word) => word[0] ?? "")
    .join("");
}
