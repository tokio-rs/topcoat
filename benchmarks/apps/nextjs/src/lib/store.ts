import { readFileSync } from "node:fs";
import path from "node:path";

export type Color =
  | "charcoal"
  | "sand"
  | "sage"
  | "navy"
  | "terracotta"
  | "ochre"
  | "plum"
  | "sky";

export interface Spec {
  label: string;
  value: string;
}

/** A review as it appears in `products.json`. */
export interface ReviewJson {
  author: string;
  rating: number;
  title: string;
  body: string;
  date: string;
}

/** A product as it appears in `products.json`. */
export interface ProductJson {
  id: number;
  slug: string;
  name: string;
  category: string;
  price_cents: number;
  compare_at_cents: number | null;
  rating: number;
  review_count: number;
  stock: number;
  color: Color;
  tagline: string;
  description: string;
  specs: Spec[];
  created_at: string;
  featured: boolean;
  related: number[];
  reviews: ReviewJson[];
}

// The loaded records keep every JSON field, for the API, and add the values parsed at startup.

export interface Review extends ReviewJson {
  /** `date`, parsed. */
  postedOn: Date;
}

export interface Product extends Omit<ProductJson, "reviews"> {
  reviews: Review[];
  /** `created_at`, parsed. */
  createdOn: Date;
  /** The record of `category`. */
  categoryRecord: Category;
  /** The records of `related`, in data order. */
  relatedProducts: Product[];
}

export interface Category {
  slug: string;
  name: string;
  description: string;
  /** Number of products in the category, counted at startup. */
  productCount: number;
}

export type OrderStatus =
  | "pending"
  | "paid"
  | "shipped"
  | "delivered"
  | "refunded"
  | "cancelled";

/** An order as it appears in `orders.json`. */
export interface OrderJson {
  id: number;
  number: string;
  customer: string;
  email: string;
  placed_at: string;
  status: OrderStatus;
  items: number;
  total_cents: number;
  country: string;
}

export interface Order extends OrderJson {
  /** `placed_at`, parsed. */
  placedAt: Date;
}

/** Aggregates over all orders, computed once at startup like a database query would. */
export interface OrderSummary {
  count: number;
  revenueCents: number;
  averageCents: number;
  refundedCents: number;
}

export const SORTS = ["newest", "price-asc", "price-desc", "rating"] as const;
export type Sort = (typeof SORTS)[number];

/** Orders two products. Ties are broken by ascending id. */
const COMPARE: Record<Sort, (a: Product, b: Product) => number> = {
  newest: (a, b) => b.createdOn.getTime() - a.createdOn.getTime() || a.id - b.id,
  "price-asc": (a, b) => a.price_cents - b.price_cents || a.id - b.id,
  "price-desc": (a, b) => b.price_cents - a.price_cents || a.id - b.id,
  rating: (a, b) =>
    ratingTenths(b) - ratingTenths(a) ||
    b.review_count - a.review_count ||
    a.id - b.id,
};

type SortedProducts = Record<Sort, Product[]>;

/**
 * The in-memory dataset with the indexes built at startup.
 *
 * Every lookup is a constant-time index or a slice, standing in for a database that already did the
 * filtering, sorting and aggregation.
 */
export class Store {
  readonly categories: Category[];
  readonly products: Product[];
  readonly featured: Product[];
  /** All orders, newest first. */
  readonly orders: Order[];
  readonly orderSummary: OrderSummary;
  private readonly categoryBySlug: Map<string, Category>;
  private readonly sortedAll: SortedProducts;
  private readonly sortedByCategory: Map<string, SortedProducts>;

  constructor(dir: string) {
    const categories = readJson<Omit<Category, "productCount">[]>(dir, "categories.json");
    const productsJson = readJson<ProductJson[]>(dir, "products.json");
    const ordersJson = readJson<OrderJson[]>(dir, "orders.json");

    productsJson.sort((a, b) => a.id - b.id);
    productsJson.forEach((product, index) => {
      if (product.id !== index + 1) {
        throw new Error(`product ids must be 1..=${index + 1}`);
      }
    });

    this.categories = categories.map((category) => ({
      ...category,
      productCount: productsJson.filter((product) => product.category === category.slug).length,
    }));
    this.categoryBySlug = new Map(this.categories.map((category) => [category.slug, category]));

    const products: Product[] = productsJson.map((product) => {
      const categoryRecord = this.categoryBySlug.get(product.category);
      if (!categoryRecord) throw new Error(`product ${product.id} has unknown category`);
      return {
        ...product,
        reviews: product.reviews.map((review) => ({ ...review, postedOn: new Date(review.date) })),
        createdOn: new Date(product.created_at),
        categoryRecord,
        relatedProducts: [],
      };
    });
    for (const product of products) {
      product.relatedProducts = product.related.map((id) => {
        const related = products[id - 1];
        if (!related) throw new Error(`product ${product.id} has unknown related ids`);
        return related;
      });
    }
    this.products = products;

    this.sortedAll = sortAll(products);
    this.sortedByCategory = new Map(
      this.categories.map((category) => [
        category.slug,
        sortAll(products.filter((product) => product.category === category.slug)),
      ]),
    );
    this.featured = products.filter((product) => product.featured);

    const orders = ordersJson.map((order) => ({ ...order, placedAt: new Date(order.placed_at) }));
    this.orders = orders.sort(
      (a, b) => b.placedAt.getTime() - a.placedAt.getTime() || b.id - a.id,
    );
    this.orderSummary = summarize(orders);
  }

  category(slug: string): Category | undefined {
    return this.categoryBySlug.get(slug);
  }

  /** Looks up a product by id. */
  product(id: number): Product | undefined {
    return this.products[id - 1];
  }

  /** Products of a category, or of all products, in the given order. */
  sorted(category: Category | undefined, sort: Sort): Product[] {
    const sorted = category ? this.sortedByCategory.get(category.slug)! : this.sortedAll;
    return sorted[sort];
  }
}

/** A product with exactly the fields of `products.json`, for the API. */
export function productJson(product: Product): ProductJson {
  const { reviews, createdOn, categoryRecord, relatedProducts, ...fields } = product;
  return { ...fields, reviews: reviews.map(({ postedOn, ...review }) => review) };
}

/** The rating in tenths of a star, for example `43` for a 4.3 rating. */
export function ratingTenths(product: Product): number {
  return Math.round(product.rating * 10);
}

/** The most a customer can add to the cart at once. */
export function maxQuantity(product: Product): number {
  return Math.min(product.stock, 10);
}

function sortAll(products: Product[]): SortedProducts {
  return Object.fromEntries(
    SORTS.map((sort) => [sort, products.toSorted(COMPARE[sort])]),
  ) as SortedProducts;
}

function summarize(orders: Order[]): OrderSummary {
  let revenueCents = 0;
  let revenueOrders = 0;
  let refundedCents = 0;
  for (const order of orders) {
    if (order.status === "paid" || order.status === "shipped" || order.status === "delivered") {
      revenueCents += order.total_cents;
      revenueOrders += 1;
    } else if (order.status === "refunded") {
      refundedCents += order.total_cents;
    }
  }
  return {
    count: orders.length,
    revenueCents,
    // Rounded half up to a cent.
    averageCents:
      revenueOrders === 0
        ? 0
        : Math.floor((revenueCents + Math.floor(revenueOrders / 2)) / revenueOrders),
    refundedCents,
  };
}

function readJson<T>(dir: string, file: string): T {
  return JSON.parse(readFileSync(path.join(dir, file), "utf8")) as T;
}

const globalForStore = globalThis as typeof globalThis & { harborStore?: Store };

/**
 * The dataset, loaded once per server process.
 *
 * Kept on `globalThis` so that every route bundle shares one instance.
 */
export function getStore(): Store {
  globalForStore.harborStore ??= new Store(
    process.env.BENCH_DATA_DIR ?? path.join(process.cwd(), "../../data"),
  );
  return globalForStore.harborStore;
}
