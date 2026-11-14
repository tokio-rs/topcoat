import type { NextRequest } from "next/server";

import { resolveListing } from "@/lib/listing";
import { getStore, type Product } from "@/lib/store";

/** The product fields included in list responses. */
function summary(product: Product) {
  return {
    id: product.id,
    slug: product.slug,
    name: product.name,
    category: product.category,
    price_cents: product.price_cents,
    compare_at_cents: product.compare_at_cents,
    rating: product.rating,
    review_count: product.review_count,
    stock: product.stock,
    color: product.color,
    tagline: product.tagline,
    created_at: product.created_at,
    featured: product.featured,
  };
}

/** `GET /api/products`: the same query as the listing page, as JSON. */
export function GET(request: NextRequest) {
  const query = request.nextUrl.searchParams;
  const result = resolveListing(getStore(), {
    category: query.get("category"),
    sort: query.get("sort"),
    page: query.get("page"),
  });
  if (!result.ok) {
    return Response.json({ error: result.error.message }, { status: result.error.status });
  }
  const { listing } = result;
  return Response.json({
    items: listing.products.map(summary),
    page: listing.page,
    page_count: listing.pageCount,
    total: listing.total,
  });
}
